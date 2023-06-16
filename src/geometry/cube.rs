use super::{GeometryData, attribute_names, GeometryFactory};

pub struct Cube {
    pub geometry: GeometryData,
}

impl Cube {
    pub fn new(size: f32) -> Self {
        let mut geometry = GeometryData::new();
            
        #[rustfmt::skip]
        let mut vertices = vec![   
            // top (0, 0, 1)
            -1.0, -1.0, 1.0,
            1.0, -1.0, 1.0,
            1.0, 1.0, 1.0,
            -1.0, 1.0, 1.0,
            // bottom (0, 0, -1)
            -1.0, 1.0, -1.0,
            1.0, 1.0, -1.0,
            1.0, -1.0, -1.0,
            -1.0, -1.0, -1.0,
            // right (1.0, 0, 0)
            1.0, -1.0, -1.0,
            1.0, 1.0, -1.0,
            1.0, 1.0, 1.0,
            1.0, -1.0, 1.0,
            // left (-1.0, 0, 0)
            -1.0, -1.0, 1.0,
            -1.0, 1.0, 1.0,
            -1.0, 1.0, -1.0,
            -1.0, -1.0, -1.0,
            // front (0, 1.0, 0)
            1.0, 1.0, -1.0,
            -1.0, 1.0, -1.0,
            -1.0, 1.0, 1.0,
            1.0, 1.0, 1.0,
            // back (0, -1.0, 0)
            1.0, -1.0, 1.0,
            -1.0, -1.0, 1.0,
            -1.0, -1.0, -1.0,
            1.0, -1.0, -1.0,
        ];

        for pos in &mut vertices {
            *pos = *pos * size;
        }

        let indices: Vec<u16> = vec![
            0, 1, 2, 2, 3, 0, // top
            4, 5, 6, 6, 7, 4, // bottom
            8, 9, 10, 10, 11, 8, // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // front
            20, 21, 22, 22, 23, 20, // back
        ];

        geometry.attributes.insert(attribute_names::POSITION, vertices);
        geometry.indices = indices;

        //Add positions and indices
        Self { geometry }
    }
}

impl GeometryFactory for Cube {
    fn texture_coords(&mut self) {

        #[rustfmt::skip]
        let texture_coords: Vec<f32>  = vec![
         0.0, 0.0,
         1.0, 0.0,
         1.0, 1.0,
         0.0, 1.0,
        // bottom (0.0, 0.0, -1)
         1.0, 0.0,
         0.0, 0.0,
         0.0, 1.0,
         1.0, 1.0,
        // right (1.0, 0.0, 0)
         0.0, 0.0,
         1.0, 0.0,
         1.0, 1.0,
         0.0, 1.0,
        // left (-1.0, 0.0, 0)
         1.0, 0.0,
         0.0, 0.0,
         0.0, 1.0,
         1.0, 1.0,
        // front (0.0, 1.0, 0)
         1.0, 0.0,
         0.0, 0.0,
         0.0, 1.0,
         1.0, 1.0,
        // back (0.0, -1.0, 0)
         0.0, 0.0,
         1.0, 0.0,
         1.0, 1.0,
         0.0, 1.0];

        self.geometry.attributes.insert( attribute_names::UV, texture_coords);

    }


    // NOT IMPL
    fn vertex_colors(&mut self) {

        let pos =  self.geometry.attributes.get(&attribute_names::POSITION).unwrap();
        let mut colors = Vec::new();

        for _ in pos{
            colors.push(1.0);
            colors.push(1.0);
            colors.push(1.0);
        }
        self.geometry.attributes.insert(attribute_names::COLOR, colors);
        
    }


    fn normals(&mut self) {
        let pos =  self.geometry.attributes.get(&attribute_names::POSITION).unwrap();
        let mut normals = Vec::new();

        for _ in pos{
            normals.push(1.0);
        }
        self.geometry.attributes.insert(attribute_names::NORMALS, normals);

    }
}
