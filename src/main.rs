use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension},
    },
};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use gpca::{
    dynamics::{implementations::cyclic::CyclicAutomaton, local::LocalDynamic},
    spaces::{
        implementations::basic::{DiscreteState, HyperGraphHeap},
        local::Stateable,
    },
    system::dynamical_system::DynamicalSystem,
    third::wgpu::{create_gpu_device, GpuDevice},
};

// Tipos de alias para el sistema dinámico
type MyHyperGraph = HyperGraphHeap<DiscreteState, (), (u32, u32)>;
type MyDynamicalSystem = DynamicalSystem<MyHyperGraph, CyclicAutomaton, DiscreteState, ()>;

#[derive(Clone, Resource)]
struct CurrentGPCA {
    model: MyDynamicalSystem,
}

#[derive(Component)]
struct MainPassCube;

#[derive(Resource)]
struct GPUContext {
    device: GpuDevice,
    image_handler: Option<Handle<Image>>,
    material_handler: Option<Handle<StandardMaterial>>,
}

impl CurrentGPCA {
    fn new() -> Self {
        const W: u32 = 2048;
        const H: u32 = 2048;

        const STATES: u32 = 6;
        const THRESH: u32 = 2;

        let mem = DiscreteState::filled_vector(W * H, STATES);
        let space = HyperGraphHeap::new_grid(&mem, W, H, ());
        let dynamic = CyclicAutomaton::new(STATES, THRESH);
        let model = DynamicalSystem::new(Box::new(space), Box::new(dynamic));

        Self { model }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugins(EguiPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .add_systems(Startup, setup)
        // .add_systems(Update, ui_example_system)
        // .add_systems(Update, render_image)
        .add_systems(Update, update_visualization)
        .run();
}

// fn ui_example_system(mut contexts: EguiContexts) {
//     egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
//         ui.label("world");
//     });
// }

fn update_visualization(
    // mut commands: Commands,
    mut context: ResMut<CurrentGPCA>,
    // mut query: Query<Entity, With<MainPassCube>>,

    // mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    gpu: ResMut<GPUContext>,
) {
    context.model.compute_sync_wgpu(&gpu.device);

    let image = images.get_mut(gpu.image_handler.as_ref().unwrap()).unwrap();

    let dynamic = context.model.dynamic() as &dyn LocalDynamic<DiscreteState, ()>;

    let states = dynamic.states();

    let data = context
        .model
        .space_state()
        .iter()
        .map(|x| x.state() as u8)
        .flat_map(|v| {
            [
                ((v as u32 * 255) / states) as u8,
                ((v as u32 * 255) / states) as u8,
                ((v as u32 * 255) / states) as u8,
                255,
            ]
        })
        .collect::<Vec<u8>>();

    // println!("data[:10]={:?}", &data[..10]);

    // let (w, h) = context.model.space().payload();

    image.data = data;

    // let size = Extent3d {
    //     width: *w,
    //     height: *h,
    //     depth_or_array_layers: 1,
    // };

    // let new_image = Image::new(
    //     size,
    //     TextureDimension::D2,
    //     data,
    //     bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
    //     RenderAssetUsages::default(),
    // );

    // let image_handle = images.add(new_image);
    // gpu.image_handler = Some(image_handle.clone());

    // let material_handle = materials.get_mut(MainPassCube).unwrap();

    materials
        .get_mut(gpu.material_handler.as_ref().unwrap())
        .unwrap()
        .base_color_texture = Some(gpu.image_handler.as_ref().unwrap().clone());
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let context = CurrentGPCA::new();
    commands.insert_resource(context.clone());

    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 20.0)),
        ..default()
    });

    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        },
        PanOrbitCamera::default(),
    ));

    //

    let (w, h) = context.model.space().payload();

    let size = Extent3d {
        width: *w,
        height: *h,
        depth_or_array_layers: 1,
    };

    let data = vec![0; (*w * *h * 4) as usize];

    let image = Image::new(
        size,
        TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    // image.texture_descriptor.usage |= TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;

    let image_handle = images.add(image);

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle.clone()),
        reflectance: 0.85,
        unlit: true,
        ..Default::default()
    });

    // let torus_handle = meshes.add(Mesh::from(Torus::new(2.0, 5.0)));
    let plane_handle = meshes.add(Mesh::from(Plane3d::new(
        Vec3::Z,
        Vec2::new(*w as f32 / 100.0, *h as f32 / 100.0),
    )));

    commands.spawn((
        PbrBundle {
            mesh: plane_handle.clone(),
            material: material_handle.clone(),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        },
        MainPassCube,
    ));

    commands.insert_resource(GPUContext {
        device: create_gpu_device(),
        material_handler: Some(material_handle.clone()),
        image_handler: Some(image_handle.clone()),
    });
}
