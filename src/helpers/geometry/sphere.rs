use super::{
    attribute_names::{self, NORMALS},
    vertex_colors_from_normals_impl, GeometryData, GeometryFactory,
};

pub struct Sphere {
    pub geometry: GeometryData,

    pub radius: f32,
    pub rings: u32,
    pub segments: u32,
}

impl Sphere {
    pub fn new(radius: f32, rings: u32, segments: u32) -> Self {
        let mut geometry = GeometryData::new();
        let mut vertices: Vec<f32> = Vec::new();

        let ring_incr = 1.0 / (rings as f32 - 1.0);
        let seg_incr = 1.0 / (segments as f32 - 1.0);

        for r in 0..rings {
            let v = r as f32 * ring_incr;
            for s in 0..segments {
                let u = 1.0 - s as f32 * seg_incr;

                let x = (std::f32::consts::PI * 2.0 * u).sin() * (std::f32::consts::PI * v).sin();
                let y = (std::f32::consts::PI * (v - 0.5)).sin();
                let z = (std::f32::consts::PI * 2.0 * u).cos() * (std::f32::consts::PI * v).sin();

                vertices.push(x * radius);
                vertices.push(y * radius);
                vertices.push(z * radius);
            }
        }

        let mut indices: Vec<u16> = Vec::new();
        for r in 0..rings - 1 {
            for s in 0..segments - 1 {
                let index = r * segments + (s + 1);
                indices.push(index as u16);

                let index = r * segments + s;
                indices.push(index as u16);

                let index = (r + 1) * segments + (s + 1);
                indices.push(index as u16);

                let index = (r + 1) * segments + s;
                indices.push(index as u16);

                let index = (r + 1) * segments + (s + 1);
                indices.push(index as u16);

                let index = r * segments + s;
                indices.push(index as u16);
            }
        }

        indices.reverse();
        geometry
            .attributes
            .insert(attribute_names::POSITION, vertices);
        geometry.indices = indices;

        //Add positions and indices
        Self {
            geometry,
            radius,
            segments,
            rings,
        }
    }

    pub fn vertex_colors_from_normal(&mut self) {
        if self.geometry.attributes.contains_key(&NORMALS) == false {
            self.normals();
        }
        vertex_colors_from_normals_impl(&mut self.geometry);
    }
}

impl GeometryFactory for Sphere {
    fn texture_coords(&mut self) {
        let ring_incr = 1.0 / (self.rings as f32 - 1.0);
        let seg_incr = 1.0 / (self.segments as f32 - 1.0);

        let mut data = Vec::new();

        for r in 0..self.rings {
            let v = r as f32 * ring_incr;
            for s in 0..self.segments {
                let u = 1.0 - s as f32 * seg_incr;

                data.push(u);
                data.push(v);
            }
        }

        self.geometry.attributes.insert(attribute_names::UV, data);
    }

    // NOT IMPL
    fn vertex_colors(&mut self) {}
    fn normals(&mut self) {
        let pos = self
            .geometry
            .attributes
            .get(&attribute_names::POSITION)
            .unwrap();
        let mut normals = Vec::new();

        for i in (0..pos.len()).step_by(3) {
            let mut v = glam::vec3(pos[i], pos[i + 1], pos[i + 2]);
            v = v.normalize();
            normals.push(v.x);
            normals.push(v.y);
            normals.push(v.z);
        }
        self.geometry
            .attributes
            .insert(attribute_names::NORMALS, normals);
    }
}
