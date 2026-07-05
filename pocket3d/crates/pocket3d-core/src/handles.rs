//! Opaque, stable resource handles (DESIGN.md §12).
//!
//! Applications never touch `wgpu::Buffer`/`wgpu::Texture`; they refer to
//! GPU/asset resources through these lightweight handles.

macro_rules! handle {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
        #[repr(transparent)]
        pub struct $name(pub u32);

        impl $name {
            /// Sentinel value meaning "no resource".
            pub const INVALID: Self = Self(u32::MAX);

            pub fn index(self) -> usize {
                self.0 as usize
            }

            pub fn is_valid(self) -> bool {
                self.0 != u32::MAX
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::INVALID
            }
        }
    };
}

handle!(
    /// Handle to a static (non-skinned) mesh.
    MeshHandle
);
handle!(
    /// Handle to a skinned mesh (vertices weighted to a skeleton).
    SkinnedMeshHandle
);
handle!(
    /// Handle to a decoded texture on the GPU.
    TextureHandle
);
handle!(
    /// Handle to a material (pipeline + texture bindings).
    MaterialHandle
);
handle!(
    /// Handle to a compiled BSP world.
    WorldHandle
);
handle!(
    /// Handle to a skeleton (joint hierarchy + inverse bind matrices).
    SkeletonHandle
);
handle!(
    /// Handle to an animation clip.
    AnimationClipHandle
);
handle!(
    /// Handle to a loaded sound.
    SoundHandle
);
