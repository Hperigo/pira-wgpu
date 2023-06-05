pub mod state;
pub use state::State;


pub mod factories;
pub use factories::BindGroupFactory;

pub use factories::texture::TextureBundle as TextureBundle;

// pub mod render_pass;
// pub use render_pass::RenderPassFactory;

// pub mod render_pipeline;
// pub use render_pipeline::RenderPipelineFactory;

// pub mod texture;
// pub use texture::{DepthTextureFactory, Texture2dFactory};
