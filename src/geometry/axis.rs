use wgpu::PrimitiveTopology;

use super::{GeometryData, GeometryFactory, attribute_names};



pub struct Axis {
    pub geometry: GeometryData,
}

impl Axis {
    pub fn new(size: f32) -> Self {
        let mut geometry = GeometryData::new();
        geometry.topology = PrimitiveTopology::LineList;

        #[rustfmt::skip]
        let vertices = vec![   
            //X Axis
            0.0, 0.0, 0.0,
            size, 0.0, 0.0,
            //X Axis
            0.0, 0.0, 0.0,
            0.0, size, 0.0,
            //X Axis
            0.0, 0.0, 0.0,
            0.0, 0.0, size,
        ];

        let indices: Vec<u16> = vec![
            0,1,2,3,4,5
        ];

        geometry.attributes.insert(attribute_names::POSITION, vertices);
        geometry.indices = indices;

        Self { geometry }
    }
}

impl GeometryFactory for Axis {
    fn texture_coords(&mut self) {

        #[rustfmt::skip]
        let texture_coords: Vec<f32>  = vec![
         0.0, 0.0,
         0.0, 0.0, 
     
         0.0, 0.0,
         0.0, 0.0,

         0.0, 0.0,
         0.0, 0.0,

        ];
         
        self.geometry.attributes.insert( attribute_names::UV, texture_coords);

    }

    fn vertex_colors(&mut self){
        #[rustfmt::skip]
        let vertex_color: Vec<f32>  = vec![
         1.0, 0.0, 0.0,
         1.0, 0.0, 0.0,
         
         0.0, 1.0, 0.0,
         0.0, 1.0, 0.0,

         0.0, 0.0, 1.0,
         0.0, 0.0, 1.0,
        ];
         
        self.geometry.attributes.insert( attribute_names::COLOR, vertex_color);
    }

    fn normals(&mut self) {}
}
