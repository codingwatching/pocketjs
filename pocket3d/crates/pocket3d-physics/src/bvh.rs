//! A compact median-split BVH over a triangle soup for raycasts and AABB
//! overlap broadphase.

use crate::geometry::{ray_aabb, ray_triangle};
use glam::Vec3;
use pocket3d_core::{Aabb, Ray, RayHit, Triangle};

const LEAF_MAX: usize = 4;

struct Node {
    bounds: Aabb,
    /// Interior node: index of the left child, or `u32::MAX` for a leaf.
    left: u32,
    /// Interior node: index of the right child (subtrees are not contiguous).
    right: u32,
    /// Leaf: range into the reordered triangle-index array.
    start: u32,
    count: u32,
}

/// Bounding-volume hierarchy over `Triangle`s.
pub struct Bvh {
    tris: Vec<Triangle>,
    /// Triangle indices reordered so leaves reference contiguous ranges.
    indices: Vec<u32>,
    nodes: Vec<Node>,
}

impl Bvh {
    pub fn build(tris: Vec<Triangle>) -> Bvh {
        let n = tris.len();
        let mut indices: Vec<u32> = (0..n as u32).collect();
        let centroids: Vec<Vec3> = tris.iter().map(|t| t.centroid()).collect();
        let mut nodes = Vec::with_capacity(n.max(1) * 2);
        if n == 0 {
            nodes.push(Node {
                bounds: Aabb::EMPTY,
                left: u32::MAX,
                right: u32::MAX,
                start: 0,
                count: 0,
            });
        } else {
            build_recursive(&tris, &centroids, &mut indices, &mut nodes, 0, n);
        }
        Bvh {
            tris,
            indices,
            nodes,
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.tris.len()
    }

    pub fn triangles(&self) -> &[Triangle] {
        &self.tris
    }

    pub fn bounds(&self) -> Aabb {
        self.nodes.first().map(|n| n.bounds).unwrap_or(Aabb::EMPTY)
    }

    /// Nearest triangle hit along `ray` within `max_t`.
    pub fn raycast(&self, ray: &Ray, max_t: f32) -> Option<RayHit> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv_dir = crate::geometry::safe_inv_dir(ray.dir);
        let mut best: Option<RayHit> = None;
        let mut best_t = max_t;
        let mut stack = [0u32; 64];
        let mut sp = 0usize;
        stack[sp] = 0;
        sp += 1;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if !ray_aabb(ray.origin, inv_dir, node.bounds.min, node.bounds.max, best_t) {
                continue;
            }
            if node.left == u32::MAX {
                for i in node.start..node.start + node.count {
                    let tri = &self.tris[self.indices[i as usize] as usize];
                    if let Some(hit) = ray_triangle(ray, tri, best_t) {
                        if hit.t < best_t {
                            best_t = hit.t;
                            best = Some(hit);
                        }
                    }
                }
            } else if sp + 2 <= stack.len() {
                let (l, r) = (node.left, node.right);
                stack[sp] = l;
                sp += 1;
                stack[sp] = r;
                sp += 1;
            }
        }
        best
    }

    /// Collect indices of triangles whose leaf node overlaps `query`.
    pub fn overlap_aabb(&self, query: &Aabb, out: &mut Vec<u32>) {
        out.clear();
        if self.nodes.is_empty() {
            return;
        }
        let mut stack = [0u32; 64];
        let mut sp = 0usize;
        stack[sp] = 0;
        sp += 1;
        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if !node.bounds.intersects(query) {
                continue;
            }
            if node.left == u32::MAX {
                for i in node.start..node.start + node.count {
                    let ti = self.indices[i as usize];
                    if self.tris[ti as usize].aabb().intersects(query) {
                        out.push(ti);
                    }
                }
            } else if sp + 2 <= stack.len() {
                let (l, r) = (node.left, node.right);
                stack[sp] = l;
                sp += 1;
                stack[sp] = r;
                sp += 1;
            }
        }
    }

    pub fn triangle(&self, index: u32) -> &Triangle {
        &self.tris[index as usize]
    }
}

fn bounds_of(tris: &[Triangle], indices: &[u32], start: usize, end: usize) -> Aabb {
    let mut b = Aabb::EMPTY;
    for &i in &indices[start..end] {
        let t = &tris[i as usize];
        b.grow(t.a);
        b.grow(t.b);
        b.grow(t.c);
    }
    b
}

fn build_recursive(
    tris: &[Triangle],
    centroids: &[Vec3],
    indices: &mut [u32],
    nodes: &mut Vec<Node>,
    start: usize,
    end: usize,
) -> u32 {
    let bounds = bounds_of(tris, indices, start, end);
    let node_index = nodes.len() as u32;
    nodes.push(Node {
        bounds,
        left: u32::MAX,
        right: u32::MAX,
        start: start as u32,
        count: (end - start) as u32,
    });

    let count = end - start;
    if count <= LEAF_MAX {
        return node_index;
    }

    // Split along the longest axis of the centroid bounds at the median.
    let mut cb = Aabb::EMPTY;
    for &i in &indices[start..end] {
        cb.grow(centroids[i as usize]);
    }
    let size = cb.size();
    let axis = if size.x >= size.y && size.x >= size.z {
        0
    } else if size.y >= size.z {
        1
    } else {
        2
    };
    if size[axis] < 1e-4 {
        return node_index; // degenerate; keep as leaf
    }

    let mid = start + count / 2;
    indices[start..end].select_nth_unstable_by(count / 2, |&a, &b| {
        centroids[a as usize][axis]
            .partial_cmp(&centroids[b as usize][axis])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let left = build_recursive(tris, centroids, indices, nodes, start, mid);
    let right = build_recursive(tris, centroids, indices, nodes, mid, end);
    // Convert this node to interior.
    nodes[node_index as usize].left = left;
    nodes[node_index as usize].right = right;
    nodes[node_index as usize].count = 0;
    node_index
}
