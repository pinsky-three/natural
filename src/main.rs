use std::f32::consts::PI;

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        texture::ImageSampler,
        view::RenderLayers, // texture::{ImageFormat, ImageSampler, ImageType},
    },
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};

use gpca::{
    dynamics::{implementations::cyclic::CyclicAutomaton, local::LocalDynamic},
    spaces::{
        implementations::basic::{DiscreteState, HyperGraphHeap},
        local::Stateable,
    },
    system::dynamical_system::DynamicalSystem,
    third::wgpu::{create_gpu_device, GpuDevice},
    // third::wgpu::create_gpu_device,
};

// const X_EXTENT: f32 = 600.;

type MyHyperGraph = HyperGraphHeap<DiscreteState, (), (u32, u32)>;
type MyDynamicalSystem = DynamicalSystem<MyHyperGraph, CyclicAutomaton, DiscreteState, ()>;

#[derive(Resource)]
struct CurrentGPCA {
    model: MyDynamicalSystem,
}

#[derive(Component)]
struct MainPassCube;

#[derive(Resource)]
struct GPUContext {
    device: GpuDevice,
}

impl CurrentGPCA {
    fn new() -> Self {
        const W: u32 = 1024;
        const H: u32 = 1024;

        const STATES: u32 = 4;
        const THRESH: u32 = 2;

        let mem = DiscreteState::filled_vector(W * H, STATES);
        let space = HyperGraphHeap::new_grid(&mem, W, H, ());

        let dynamic = CyclicAutomaton::new(STATES, THRESH);

        let model = DynamicalSystem::new(Box::new(space), Box::new(dynamic));

        Self { model }
    }
}

fn main() {
    // let mut app = ;

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, ui_example_system)
        .add_systems(Update, render_image)
        .run();
}

fn ui_example_system(mut contexts: EguiContexts) {
    egui::Window::new("Hello").show(contexts.ctx_mut(), |ui| {
        ui.label("world");
    });
}

fn render_image(
    gpu: Res<GPUContext>,
    mut commands: Commands,
    mut context: ResMut<CurrentGPCA>,

    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    context.model.compute_sync_wgpu(&gpu.device);

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

    let size = Extent3d {
        width: 1024,
        height: 1024,
        depth_or_array_layers: 1,
    };

    // This is the texture that will be rendered to.
    let image = Image::new(
        size,
        TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    let image_handle = images.add(image);

    // let first_pass_layer = RenderLayers::layer(1);

    // commands.spawn((
    //     Camera3dBundle {
    //         camera: Camera {
    //             order: -1,
    //             target: image_handle.clone().into(),
    //             clear_color: Color::WHITE.into(),
    //             ..default()
    //         },
    //         transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
    //             .looking_at(Vec3::ZERO, Vec3::Y),
    //         ..default()
    //     },
    //     first_pass_layer,
    // ));

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // let cube_size = 4.0;

    let torus_handle = meshes.add(Torus::new(3.0, 6.0));

    commands.spawn((
        PbrBundle {
            mesh: torus_handle,
            material: material_handle,
            transform: Transform::from_xyz(0.0, 0.0, 1.5)
                .with_rotation(Quat::from_rotation_x(-PI / 5.0)),
            ..default()
        },
        MainPassCube,
    ));
}

fn setup(
    mut commands: Commands,
    // mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.insert_resource(CurrentGPCA::new());
    commands.insert_resource(GPUContext {
        device: create_gpu_device(),
    });

    commands.spawn((
        PointLightBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
            ..default()
        },
        RenderLayers::layer(0).with(1),
    ));

    // The main pass camera.
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // commands.spawn(Camera2dBundle::default());

    // let shapes = [
    //     Mesh2dHandle(meshes.add(Circle { radius: 35.0 })),
    //     Mesh2dHandle(meshes.add(Ellipse::new(25.0, 50.0))),
    //     Mesh2dHandle(meshes.add(Capsule2d::new(25.0, 50.0))),
    //     Mesh2dHandle(meshes.add(Rectangle::new(50.0, 100.0))),
    //     Mesh2dHandle(meshes.add(RegularPolygon::new(50.0, 6))),
    //     Mesh2dHandle(meshes.add(Triangle2d::new(
    //         Vec2::Y * 50.0,
    //         Vec2::new(-50.0, -50.0),
    //         Vec2::new(50.0, -50.0),
    //     ))),
    // ];

    // let num_shapes = shapes.len();

    // for (i, shape) in shapes.into_iter().enumerate() {
    //     // Distribute colors evenly across the rainbow.
    //     let color = Color::hsl(360. * i as f32 / num_shapes as f32, 0.95, 0.7);

    //     commands.spawn(MaterialMesh2dBundle {
    //         mesh: shape,
    //         material: materials.add(color),
    //         transform: Transform::from_xyz(
    //             // Distribute shapes from -X_EXTENT to +X_EXTENT.
    //             -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
    //             0.0,
    //             0.0,
    //         ),
    //         ..default()
    //     });
    // }
}

// async fn ff() {
//     const W: u32 = 2048;
//     const H: u32 = 1024;

//     const STATES: u32 = 4;
//     const THRESH: u32 = 2;

//     let _device = create_gpu_device();

//     let mem = DiscreteState::filled_vector(W * H, STATES);
//     let space = HyperGraphHeap::new_grid(&mem, W, H, ());

//     let dynamic = CyclicAutomaton::new(STATES, THRESH);

//     let mut system = DynamicalSystem::new(Box::new(space), Box::new(dynamic));

//     for _ in tqdm!(0..500) {
//         system.compute_sync_wgpu(&_device);
//         // system.compute_sync();
//     }

//     save_space_as_image(&system, colorous::PLASMA);
// }
