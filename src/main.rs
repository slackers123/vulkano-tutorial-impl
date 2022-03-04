use std::sync::Arc;

use vulkano::instance::{Instance, InstanceExtensions};
use vulkano::Version;
use vulkano::device::physical::{PhysicalDevice, QueueFamily};
use vulkano::device::{Device, DeviceExtensions, Features, QueuesIter};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::pipeline::{ComputePipeline, Pipeline, PipelineBindPoint};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::sync;
use vulkano::sync::GpuFuture;
use vulkano::image::{ImageDimensions, StorageImage, view::ImageView};
use vulkano::format::{Format, ClearValue};
use image::{ImageBuffer, Rgba};

fn create_instance() -> Arc<Instance> {
    let instance = Instance::new(None, Version::V1_5, &InstanceExtensions::none(), None)
    .expect("failed to create instance");

    instance
}

fn create_physical(instance: &Arc<Instance>) -> PhysicalDevice {
    let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");

    physical
}

fn create_queue_family<'a>(physical: &'a PhysicalDevice) -> QueueFamily<'a> {
    for family in physical.queue_families() {
        println!("Found a queue family with {:?} queue(s)", family.queues_count());
    }    

    let queue_family: QueueFamily<'a> = physical.queue_families()
        .find(|&q| q.supports_graphics())
        .expect("couldn't find a graphical queue family");

    queue_family
}

fn create_device_and_queues(physical: &PhysicalDevice, queue_family: &QueueFamily) -> (Arc<Device>, QueuesIter) {
    let (device, queues) = {
        Device::new(*physical, &Features::none(), &DeviceExtensions::none(),
                    [(*queue_family, 0.5)].iter().cloned()).expect("failed to create device")
    };

    (device, queues)
}

struct VkData<'a> {
    instance: Arc<Instance>,
    physical: PhysicalDevice<'a>,
    queue_family: QueueFamily<'a>,
    device: Arc<Device>,
    queues: QueuesIter,
}

// struct App<'a> {
//     instance: Arc<Instance>,
//     physical: PhysicalDevice<'a>,
//     queue_family: QueueFamily<'a>,
//     device: Arc<Device>,
//     queues: QueuesIter,
// }

// impl App<'_> {
//     pub fn new() -> Self {
//         let instance = create_instance();
//         let physical = create_physical(&instance);
//         let queue_family = create_queue_family(&physical);

//         let (device, mut queues) = create_device_and_queues(&physical, &queue_family);

//         App {
//             instance,
//             physical,
//             queue_family,
//             device,
//             queues,
//         }
//     }
// }

fn main() {
    let instance = create_instance();

    let physical = create_physical(&instance);

    let queue_family = create_queue_family(&physical);

    let (device, mut queues) = create_device_and_queues(&physical, &queue_family);

    let queue = queues.next().unwrap();

    mod cs {
        vulkano_shaders::shader!{
            ty: "compute",
            path: "src/shaders/shader.glsl",
        }
    }

    let shader = cs::load(device.clone()).expect("failed to create shader module");

    let vk_data = VkData {
        instance: instance.clone(),
        physical,
        queue_family,
        device,
        queues,
    };

    // Image creation

    let compute_pipeline = ComputePipeline::new(
        vk_data.device.clone(),
        shader.entry_point("main").unwrap(),
        &(),
        None,
        |_| {},
    )
    .expect("failed to create compute pipeline");

    let image = StorageImage::new(
        vk_data.device.clone(),
        ImageDimensions::Dim2d {
            width: 1024,
            height: 1024,
            array_layers: 1,
        },
        Format::R8G8B8A8_UNORM,
        Some(queue.family()),
    )
    .unwrap();
    
    let view = ImageView::new(image.clone()).unwrap();
    let layout = compute_pipeline
        .layout()
        .descriptor_set_layouts()
        .get(0)
        .unwrap();
    let set = PersistentDescriptorSet::new(
        layout.clone(),
        [WriteDescriptorSet::image_view(0, view.clone())], // 0 is the binding
    )
    .unwrap();

    let buf = CpuAccessibleBuffer::from_iter(
        vk_data.device.clone(),
        BufferUsage::all(),
        false,
        (0..1024 * 1024 * 4).map(|_| 0u8),
    )
    .expect("failed to create buffer");

    let mut builder = AutoCommandBufferBuilder::primary(
        vk_data.device.clone(),
        queue.family(),
        CommandBufferUsage::OneTimeSubmit,
    )
    .unwrap();
    builder
        .bind_pipeline_compute(compute_pipeline.clone())
        .bind_descriptor_sets(
            PipelineBindPoint::Compute,
            compute_pipeline.layout().clone(),
            0,
            set,
        )
        .dispatch([1024 / 8, 1024 / 8, 1])
        .unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone())
        .unwrap();

    let command_buffer = builder.build().unwrap();

    let future = sync::now(vk_data.device.clone())
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_signal_fence_and_flush()
        .unwrap();

    future.wait(None).unwrap();

    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(1024, 1024, &buffer_content[..]).unwrap();
    image.save("output/image.png").unwrap();


    println!("stored image");
    
}