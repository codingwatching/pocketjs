//! The `RenderDevice` resource-creation contract.
//!
//! Applications register GPU resources through these methods and receive opaque
//! [`pocket3d_core`] handles back. They never touch `wgpu` types directly
//! (DESIGN.md §12).

use pocket3d_core::{
    material::MaterialDesc,
    mesh::{MeshData, SkinnedVertex, StaticVertex, WorldVertex},
    texture::TextureData,
    MaterialHandle, MeshHandle, SkinnedMeshHandle, TextureHandle, WorldHandle,
};

/// The render-ready payload of a compiled BSP world, borrowed for upload.
pub struct WorldUpload<'a> {
    pub mesh: &'a MeshData<WorldVertex>,
    /// Material table; `Submesh::material` indexes into this.
    pub materials: &'a [MaterialDesc],
    /// Base textures; `MaterialDesc::base_texture` indexes into this.
    pub textures: &'a [TextureData],
    /// One combined lightmap atlas, if the map had lighting.
    pub lightmap_atlas: Option<&'a TextureData>,
}

/// Backend-agnostic resource creation. Implemented by `pocket3d-render-wgpu`.
pub trait RenderDevice {
    /// Upload a compiled BSP world (geometry + materials + textures + lightmap).
    fn upload_world(&mut self, world: &WorldUpload<'_>) -> WorldHandle;

    /// Upload a static (unskinned) mesh.
    fn upload_static_mesh(&mut self, mesh: &MeshData<StaticVertex>) -> MeshHandle;

    /// Upload a skinned mesh (vertices weighted to joints).
    fn upload_skinned_mesh(&mut self, mesh: &MeshData<SkinnedVertex>) -> SkinnedMeshHandle;

    /// Upload a texture and return its handle.
    fn upload_texture(&mut self, tex: &TextureData) -> TextureHandle;

    /// Create a material referencing an already-uploaded base texture.
    fn create_material(&mut self, desc: &MaterialDesc, base: TextureHandle) -> MaterialHandle;
}
