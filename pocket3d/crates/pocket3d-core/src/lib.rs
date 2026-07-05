//! `pocket3d-core` — platform-neutral runtime types shared by every Pocket3D
//! crate: math conventions, geometry primitives, entity storage, resource
//! handles, time, input snapshots, cameras, events, and world stores.
//!
//! This crate has **no** engine/renderer/physics dependency. It defines the
//! contract that the renderer, physics, KCC, animation, and application layers
//! all agree on. See `DESIGN.md` §6–§8.
//!
//! ## Coordinate system (DESIGN.md §8)
//!
//! Pocket3D uses a **Z-up, right-handed** world:
//! - `+X` = right / east
//! - `+Y` = forward / north
//! - `+Z` = up
//! - 1 world unit = 1 BSP map unit
//!
//! Simulation, physics, BSP entities, and debug tools all stay in these world
//! coordinates. Only the renderer maps into clip space.

pub mod camera;
pub mod events;
pub mod geom;
pub mod handles;
pub mod input;
pub mod material;
pub mod math;
pub mod mesh;
pub mod texture;
pub mod time;
pub mod world;

pub use camera::Camera;
pub use events::{Event, EventQueue, HitEvent, HitKind};
pub use geom::{Aabb, Capsule, Plane, Ray, RayHit, Triangle};
pub use handles::{
    AnimationClipHandle, MaterialHandle, MeshHandle, SkeletonHandle, SkinnedMeshHandle,
    SoundHandle, TextureHandle, WorldHandle,
};
pub use input::{Button, InputSnapshot, Key};
pub use material::{MaterialDesc, MaterialKind};
pub use math::Transform;
pub use mesh::{CollisionMesh, MeshData, SkinnedVertex, StaticVertex, Submesh, WorldVertex};
pub use texture::TextureData;
pub use time::{FixedClock, TickInfo};
pub use world::{
    AudioEmitter, CameraStore, ColliderRef, ColliderStore, EntityId, EntityStore, KccState,
    KccStore, MeshRenderer, MeshRendererStore, ScriptBinding, ScriptBindingStore, SkinnedMesh,
    SkinnedMeshStore, TransformStore, World,
};

/// Re-export of the math library so downstream crates share one `glam` version.
pub use glam;
pub use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
