use wgpu::BufferAddress;

pub mod pbr;
pub mod shadeless;
pub mod sky;

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

impl Default for ModelUniform {
 fn default() -> Self {
     ModelUniform{
        model_matrix : glam::Mat4::IDENTITY,
     }
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

    model_buffer
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

fn ceil_to_next_multiple(val: u64, step: u64) -> u64 {
    let rmder = if val % step == 0 { 0 } else { 1 };
    let divide_and_ceil = val / step + rmder;

    divide_and_ceil * step
}

pub fn create_uniform_buffer_stride<T>(count: u64, device: &wgpu::Device) -> wgpu::Buffer {
    let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;

    let stride = ceil_to_next_multiple(
        std::mem::size_of::<T>().try_into().unwrap(),
        uniform_alignment,
    ) as BufferAddress;

    println!(
        "Uniform alignment: {}, {}, {}",
        uniform_alignment,
        std::mem::size_of::<T>(),
        stride,
    );
    let model_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Model Matrix"),
        size: ((std::mem::size_of::<T>() as u64 + stride * count) as wgpu::BufferAddress),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
        mapped_at_creation: false,
    });

    model_buffer
}

pub fn write_uniform_buffer_stride<T>(
    data: &[T],
    buffer: &wgpu::Buffer,
    queue: &wgpu::Queue,
    device: &wgpu::Device,
) {
    let uniform_alignment = device.limits().min_uniform_buffer_offset_alignment as u64;

    let size_of_t = std::mem::size_of::<T>();

    let stride =
        ceil_to_next_multiple(size_of_t.try_into().unwrap(), uniform_alignment) as BufferAddress;

    for i in 0..data.len() {
        let ptr = (data as *const _) as *const u8;
        let data_bytes = unsafe { std::slice::from_raw_parts(ptr, size_of_t) };

        let offset = stride * i as u64;
        println!("Writting buffer at: {}", offset);
        queue.write_buffer(buffer, offset, data_bytes);
    }
}
