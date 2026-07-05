//! Entity storage and component stores (DESIGN.md §7).
//!
//! v0 deliberately avoids a large ECS. We use `slotmap` for a generational
//! entity arena plus one `SecondaryMap` per component store. This gives stable
//! entity IDs and O(1) component access without coupling to Bevy/Avian.

use crate::geom::Capsule;
use crate::handles::{
    MaterialHandle, MeshHandle, SkeletonHandle, SkinnedMeshHandle, SoundHandle,
};
use crate::math::Transform;
use glam::Vec3;
use slotmap::{new_key_type, SecondaryMap, SlotMap};

new_key_type! {
    /// A stable, generational entity identifier.
    pub struct EntityId;
}

/// Per-entity metadata held by the arena.
#[derive(Clone, Debug, Default)]
pub struct EntityMeta {
    pub name: Option<String>,
}

/// A static or skinned mesh to draw at the entity's transform.
#[derive(Clone, Copy, Debug)]
pub struct MeshRenderer {
    pub mesh: MeshHandle,
    pub material: MaterialHandle,
    pub visible: bool,
}

/// A skinned mesh + skeleton + the animation runtime slot driving it.
#[derive(Clone, Copy, Debug)]
pub struct SkinnedMesh {
    pub mesh: SkinnedMeshHandle,
    pub skeleton: SkeletonHandle,
    pub material: MaterialHandle,
    /// Index into the animation runtime's pose/instance buffer.
    pub pose_slot: u32,
    pub visible: bool,
}

/// Collision shape kinds a character/query can use.
#[derive(Clone, Copy, Debug)]
pub enum ColliderShape {
    Capsule { radius: f32, height: f32 },
    Sphere { radius: f32 },
    Box { half_extents: Vec3 },
}

/// A reference to an entity's collision shape (offset in local space).
#[derive(Clone, Copy, Debug)]
pub struct ColliderRef {
    pub shape: ColliderShape,
    pub offset: Vec3,
    /// Whether hitscan queries should test this collider.
    pub is_hittable: bool,
}

/// Kinematic character controller runtime state stored per-entity.
#[derive(Clone, Copy, Debug)]
pub struct KccState {
    pub velocity: Vec3,
    pub grounded: bool,
    pub radius: f32,
    pub height: f32,
    pub step_height: f32,
    pub slope_limit_deg: f32,
}

impl Default for KccState {
    fn default() -> Self {
        Self {
            velocity: Vec3::ZERO,
            grounded: false,
            radius: 16.0,
            height: 72.0,
            step_height: 18.0,
            slope_limit_deg: 45.0,
        }
    }
}

impl KccState {
    /// The capsule for this controller given a feet position.
    pub fn capsule_at(&self, feet: Vec3) -> Capsule {
        Capsule::from_base_height(feet, self.height, self.radius)
    }
}

/// A positional audio source (position taken from the entity transform).
#[derive(Clone, Copy, Debug, Default)]
pub struct AudioEmitter {
    pub last_sound: Option<SoundHandle>,
    pub volume: f32,
}

/// A binding to script-owned data for this entity.
#[derive(Clone, Copy, Debug, Default)]
pub struct ScriptBinding {
    pub script_id: u32,
}

pub type EntityStore = SlotMap<EntityId, EntityMeta>;
pub type TransformStore = SecondaryMap<EntityId, Transform>;
pub type MeshRendererStore = SecondaryMap<EntityId, MeshRenderer>;
pub type SkinnedMeshStore = SecondaryMap<EntityId, SkinnedMesh>;
pub type KccStore = SecondaryMap<EntityId, KccState>;
pub type ColliderStore = SecondaryMap<EntityId, ColliderRef>;
pub type CameraStore = SecondaryMap<EntityId, crate::camera::Camera>;
pub type AudioEmitterStore = SecondaryMap<EntityId, AudioEmitter>;
pub type ScriptBindingStore = SecondaryMap<EntityId, ScriptBinding>;

/// The engine world: a generational entity arena plus component stores.
#[derive(Default)]
pub struct World {
    pub entities: EntityStore,
    pub transforms: TransformStore,
    pub mesh_renderers: MeshRendererStore,
    pub skinned_meshes: SkinnedMeshStore,
    pub kcc: KccStore,
    pub colliders: ColliderStore,
    pub cameras: CameraStore,
    pub audio_emitters: AudioEmitterStore,
    pub script_bindings: ScriptBindingStore,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawn a new entity with an identity transform and optional name.
    pub fn spawn(&mut self, name: Option<&str>) -> EntityId {
        let id = self.entities.insert(EntityMeta {
            name: name.map(|s| s.to_string()),
        });
        self.transforms.insert(id, Transform::IDENTITY);
        id
    }

    pub fn is_alive(&self, id: EntityId) -> bool {
        self.entities.contains_key(id)
    }

    /// Remove an entity and all of its components.
    pub fn despawn(&mut self, id: EntityId) {
        self.entities.remove(id);
        self.transforms.remove(id);
        self.mesh_renderers.remove(id);
        self.skinned_meshes.remove(id);
        self.kcc.remove(id);
        self.colliders.remove(id);
        self.cameras.remove(id);
        self.audio_emitters.remove(id);
        self.script_bindings.remove(id);
    }

    pub fn transform(&self, id: EntityId) -> Option<&Transform> {
        self.transforms.get(id)
    }

    pub fn set_transform(&mut self, id: EntityId, t: Transform) {
        self.transforms.insert(id, t);
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}
