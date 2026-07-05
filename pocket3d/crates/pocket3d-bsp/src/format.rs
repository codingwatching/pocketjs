//! GoldSrc BSP version 30 on-disk layout (DESIGN.md §10).
//!
//! Header: `i32 version` (== 30) followed by 15 lump directory entries, each
//! `{ i32 offset; i32 length }`. Lump order is fixed (see [`LumpId`]). Verified
//! against real `de_dust2.bsp` / `de_nuke.bsp` etc.

use crate::reader::Reader;
use anyhow::{bail, Context, Result};
use glam::Vec3;

/// GoldSrc / Half-Life 1 BSP version.
pub const BSP_VERSION_GOLDSRC: i32 = 30;

pub const NUM_LUMPS: usize = 15;

/// Lump indices in the header directory.
#[derive(Clone, Copy, Debug)]
pub enum LumpId {
    Entities = 0,
    Planes = 1,
    Textures = 2,
    Vertices = 3,
    Visibility = 4,
    Nodes = 5,
    TexInfo = 6,
    Faces = 7,
    Lighting = 8,
    ClipNodes = 9,
    Leaves = 10,
    MarkSurfaces = 11,
    Edges = 12,
    SurfEdges = 13,
    Models = 14,
}

pub const LUMP_NAMES: [&str; NUM_LUMPS] = [
    "ENTITIES",
    "PLANES",
    "TEXTURES",
    "VERTICES",
    "VISIBILITY",
    "NODES",
    "TEXINFO",
    "FACES",
    "LIGHTING",
    "CLIPNODES",
    "LEAVES",
    "MARKSURFACES",
    "EDGES",
    "SURFEDGES",
    "MODELS",
];

#[derive(Clone, Copy, Debug, Default)]
pub struct Lump {
    pub offset: u32,
    pub length: u32,
}

/// The parsed header: version + lump directory.
#[derive(Clone, Debug)]
pub struct Header {
    pub version: i32,
    pub lumps: [Lump; NUM_LUMPS],
}

impl Header {
    pub fn parse(data: &[u8]) -> Result<Header> {
        let mut r = Reader::new(data);
        let version = r.i32().context("reading BSP version")?;
        let mut lumps = [Lump::default(); NUM_LUMPS];
        for l in lumps.iter_mut() {
            l.offset = r.u32()?;
            l.length = r.u32()?;
        }
        Ok(Header { version, lumps })
    }

    pub fn lump(&self, id: LumpId) -> Lump {
        self.lumps[id as usize]
    }

    /// Borrow the raw bytes of a lump, bounds-checked.
    pub fn lump_bytes<'a>(&self, data: &'a [u8], id: LumpId) -> Result<&'a [u8]> {
        let l = self.lump(id);
        // Widen to usize BEFORE adding so a hostile `offset + length` can't wrap
        // a u32 and defeat the bounds check below.
        let start = l.offset as usize;
        let end = start.saturating_add(l.length as usize);
        if end > data.len() {
            bail!(
                "lump {} out of range: {}..{} > {}",
                LUMP_NAMES[id as usize],
                start,
                end,
                data.len()
            );
        }
        Ok(&data[start..end])
    }
}

/// `dplane_t` (20 bytes).
#[derive(Clone, Copy, Debug)]
pub struct Plane {
    pub normal: Vec3,
    pub dist: f32,
    pub kind: i32,
}

/// `dedge_t` (4 bytes): two vertex indices.
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub v: [u16; 2],
}

/// `texinfo_t` (40 bytes): the S/T projection vectors + miptex index + flags.
#[derive(Clone, Copy, Debug)]
pub struct TexInfo {
    /// `s = dot(pos, s_axis) + s_offset`
    pub s_axis: Vec3,
    pub s_offset: f32,
    pub t_axis: Vec3,
    pub t_offset: f32,
    pub miptex: u32,
    pub flags: i32,
}

/// GoldSrc `TEX_SPECIAL` flag: surface has no lightmap (sky, nodraw, etc.).
pub const TEX_SPECIAL: i32 = 1;

/// `dface_t` (20 bytes).
#[derive(Clone, Copy, Debug)]
pub struct Face {
    pub plane: u16,
    pub side: i16,
    pub first_edge: u32,
    pub num_edges: u16,
    pub texinfo: u16,
    pub styles: [u8; 4],
    /// Byte offset into the LIGHTING lump, or -1 for no lightmap.
    pub light_ofs: i32,
}

/// `dmodel_t` (64 bytes). Model 0 is the world; models 1.. are brush entities.
#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub mins: Vec3,
    pub maxs: Vec3,
    pub origin: Vec3,
    pub head_nodes: [i32; 4],
    pub vis_leafs: i32,
    pub first_face: u32,
    pub num_faces: u32,
}

/// The decoded lump arrays we actually use for mesh/collision/entities.
/// Visibility, nodes, leaves, clipnodes, marksurfaces are read for the inspector
/// summary but not needed for the v0 renderer/collision path.
pub struct RawBsp {
    pub header: Header,
    pub entities_text: String,
    pub planes: Vec<Plane>,
    pub vertices: Vec<Vec3>,
    pub edges: Vec<Edge>,
    pub surf_edges: Vec<i32>,
    pub texinfos: Vec<TexInfo>,
    pub faces: Vec<Face>,
    pub lighting: Vec<u8>,
    pub models: Vec<Model>,
    /// Raw TEXTURES lump bytes (miptex table); decoded lazily with WADs.
    pub textures_lump: Vec<u8>,
}

impl RawBsp {
    pub fn parse(data: &[u8]) -> Result<RawBsp> {
        let header = Header::parse(data)?;
        if header.version != BSP_VERSION_GOLDSRC {
            bail!(
                "unsupported BSP version {} (expected {} = GoldSrc/HL1)",
                header.version,
                BSP_VERSION_GOLDSRC
            );
        }

        let entities_text = {
            let b = header.lump_bytes(data, LumpId::Entities)?;
            // The entity lump is NUL-terminated ASCII.
            let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
            String::from_utf8_lossy(&b[..end]).into_owned()
        };

        let planes = parse_array(data, &header, LumpId::Planes, 20, |r| {
            Ok(Plane {
                normal: r.vec3()?,
                dist: r.f32()?,
                kind: r.i32()?,
            })
        })?;

        let vertices = parse_array(data, &header, LumpId::Vertices, 12, |r| r.vec3())?;

        let edges = parse_array(data, &header, LumpId::Edges, 4, |r| {
            Ok(Edge {
                v: [r.u16()?, r.u16()?],
            })
        })?;

        let surf_edges = parse_array(data, &header, LumpId::SurfEdges, 4, |r| r.i32())?;

        let texinfos = parse_array(data, &header, LumpId::TexInfo, 40, |r| {
            Ok(TexInfo {
                s_axis: r.vec3()?,
                s_offset: r.f32()?,
                t_axis: r.vec3()?,
                t_offset: r.f32()?,
                miptex: r.u32()?,
                flags: r.i32()?,
            })
        })?;

        let faces = parse_array(data, &header, LumpId::Faces, 20, |r| {
            Ok(Face {
                plane: r.u16()?,
                side: r.i16()?,
                first_edge: r.u32()?,
                num_edges: r.u16()?,
                texinfo: r.u16()?,
                styles: [r.u8()?, r.u8()?, r.u8()?, r.u8()?],
                light_ofs: r.i32()?,
            })
        })?;

        let lighting = header.lump_bytes(data, LumpId::Lighting)?.to_vec();

        let models = parse_array(data, &header, LumpId::Models, 64, |r| {
            Ok(Model {
                mins: r.vec3()?,
                maxs: r.vec3()?,
                origin: r.vec3()?,
                head_nodes: [r.i32()?, r.i32()?, r.i32()?, r.i32()?],
                vis_leafs: r.i32()?,
                first_face: r.u32()?,
                num_faces: r.u32()?,
            })
        })?;

        let textures_lump = header.lump_bytes(data, LumpId::Textures)?.to_vec();

        Ok(RawBsp {
            header,
            entities_text,
            planes,
            vertices,
            edges,
            surf_edges,
            texinfos,
            faces,
            lighting,
            models,
            textures_lump,
        })
    }
}

/// Read a fixed-stride array out of a lump.
fn parse_array<T>(
    data: &[u8],
    header: &Header,
    id: LumpId,
    stride: usize,
    mut read_one: impl FnMut(&mut Reader) -> Result<T>,
) -> Result<Vec<T>> {
    let bytes = header.lump_bytes(data, id)?;
    if stride == 0 {
        return Ok(Vec::new());
    }
    let count = bytes.len() / stride;
    let mut out = Vec::with_capacity(count);
    let mut r = Reader::new(bytes);
    for _ in 0..count {
        out.push(read_one(&mut r).with_context(|| {
            format!("parsing element of lump {}", LUMP_NAMES[id as usize])
        })?);
    }
    Ok(out)
}
