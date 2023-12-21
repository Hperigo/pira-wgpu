use std::collections::HashMap;
use wgpu::PrimitiveTopology;

pub mod axis;
pub mod cube;
pub mod sphere;

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

pub fn vertex_colors_from_normals_impl(geometry: &mut GeometryData) {
    let normals: &Vec<f32> = geometry.attributes.get(&attribute_names::NORMALS).unwrap();

    let len = (normals.len() / 3) * 4;
    let mut colors = vec![1.0; len];

    let mut normal_index: usize = 0;
    for i in (0..colors.len()).step_by(4) {
        colors[i] = normals[normal_index];
        colors[i + 1] = normals[normal_index + 1];
        colors[i + 2] = normals[normal_index + 2];
        colors[i + 3] = 1.0;

        normal_index += 3;
    }

    geometry.attributes.insert(attribute_names::COLOR, colors);
}
