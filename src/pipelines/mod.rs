pub mod pbr;
pub mod shadeless;

#[repr(C, align(256))]
#[derive(Clone, Copy)]
pub struct ViewUniform {
    pub view_pespective_matrix: glam::Mat4,
    pub view_matrix: glam::Mat4,
    pub perspective_matrix: glam::Mat4,
    pub camera_position: glam::Vec3,
}

pub fn create_global_uniform(device: &wgpu::Device) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("GlobalUniform"),
        size: std::mem::size_of::<ViewUniform>() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });

    return buffer;
}

pub fn write_global_uniform_buffer(
    camera_matrix: glam::Mat4,
    buffer: &wgpu::Buffer,
    queue: &wgpu::Queue,
) {
    queue.write_buffer(buffer, 0, unsafe {
        std::slice::from_raw_parts(
            // camera_matrix.as_ptr() as *const u8,
            camera_matrix.as_ref().as_ptr() as *const u8,
            std::mem::size_of::<glam::Mat4>(),
        )
    });
}

#[repr(C, align(256))]
#[derive(Clone, Copy)]
pub struct ModelUniform {
    pub model_matrix: glam::Mat4,
}

impl ModelUniform {
    pub fn new(mat: glam::Mat4) -> Self {
        Self { model_matrix: mat }
    }
}

pub fn create_uniform_buffer<T>(count: usize, device: &wgpu::Device) -> wgpu::Buffer {
    let uniform_alignment =
        device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;

    println!(
        "Uniform alignment: {}, {}",
        uniform_alignment,
        std::mem::size_of::<T>()
    );
    let model_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Model Matrix"),
        size: ((std::mem::size_of::<T>() * count) as wgpu::BufferAddress) * uniform_alignment,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });

    return model_buffer;
}

pub fn write_uniform_buffer<T>(
    data: &[T],
    buffer: &wgpu::Buffer,
    queue: &wgpu::Queue,
    _device: &wgpu::Device,
) {
    // let uniform_alignment =
    //     device.limits().min_uniform_buffer_offset_alignment as wgpu::BufferAddress;

    let len = data.len() * std::mem::size_of::<T>();

    // println!("T: {}", std::mem::size_of::<T>());
    // println!("Len: {}", len);
    queue.write_buffer(&buffer, 0, unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, len)
    });
}
