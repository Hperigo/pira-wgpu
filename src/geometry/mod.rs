use std::collections::HashMap;

pub mod cube;
pub use cube::Cube;

pub mod axis;
pub use axis::Axis;

pub mod sphere;
pub use sphere::Sphere;

use wgpu::PrimitiveTopology;

pub mod attribute_names {
    pub type AttributeIndex = u32;

    pub const POSITION: AttributeIndex = 0;
    pub const UV: AttributeIndex = 1;
    pub const COLOR: AttributeIndex = 2;
    pub const NORMALS: AttributeIndex = 3;
}

pub struct GeometryData {
    pub attributes: HashMap<attribute_names::AttributeIndex, Vec<f32>>,
    pub indices: Vec<u16>,
    pub topology: PrimitiveTopology,
}

impl GeometryData {
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
            indices: Vec::new(),
            topology: PrimitiveTopology::TriangleList,
        }
    }
}

pub trait GeometryFactory {
    fn texture_coords(&mut self);
    fn vertex_colors(&mut self);
    fn normals(&mut self);
}
