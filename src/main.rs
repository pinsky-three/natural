use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension},
    },
};
// use bevy_egui::{egui, EguiContexts, EguiPlugin};
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
use rand::{self, Rng};
use rayon::iter::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

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
    // device: Arc<GpuDevice>,
    image_handler: Option<Handle<Image>>,
    material_handler: Option<Handle<StandardMaterial>>,
}

#[derive(Resource)]
struct UiState {
    ca_states: u32,
    ca_thresh: u32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            ca_states: 5,
            ca_thresh: 3,
        }
    }
}

impl CurrentGPCA {
    fn new() -> Self {
        const W: u32 = 512;
        const H: u32 = 512;

        const STATES: u32 = 10;
        const THRESH: u32 = 2;

        let mem = (0..W * H)
            .into_par_iter()
            .map(|_| DiscreteState::from_state(rand::thread_rng().gen_range(0..STATES)))
            .collect();

        let space = HyperGraphHeap::new_grid(&mem, W, H, ());
        let dynamic = CyclicAutomaton::new(STATES, THRESH);
        let model = DynamicalSystem::new(Box::new(space), Box::new(dynamic));

        Self { model }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "GPCA".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .init_resource::<UiState>()
        // .add_plugins(EguiPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .add_systems(Startup, setup)
        // .add_systems(Update, ui_example_system)
        // .add_systems(Update, render_image)
        .add_systems(Update, update_visualization)
        .run();
}

// fn ui_example_system(
//     mut contexts: EguiContexts,
//     mut context: ResMut<CurrentGPCA>,
//     mut ui_state: ResMut<UiState>,
// ) {
//     egui::Window::new("Cyclic Cellular Automata").show(contexts.ctx_mut(), |ui| {
//         // ui.label("world");

//         ui.horizontal(|ui| {
//             ui.label("states");
//             ui.add(egui::Slider::new(&mut ui_state.ca_states, 1..=32));
//         });

//         ui.horizontal(|ui| {
//             ui.label("threshold");
//             ui.add(egui::Slider::new(&mut ui_state.ca_thresh, 1..=8));
//         });

//         ui.horizontal(|ui| {
//             if ui.button("reset").clicked() {
//                 let dynamic = context.model.dynamic() as &dyn LocalDynamic<DiscreteState, ()>;
//                 let states = dynamic.states();

//                 // let s = ;

//                 context.model.update_space(|mem| {
//                     mem.par_iter_mut()
//                         .for_each(|x| x.set_state(rand::thread_rng().gen_range(0..states)));
//                 });
//             }
//         });
//     });
// }

fn update_visualization(
    mut context: ResMut<CurrentGPCA>,

    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,

    ui_state: ResMut<UiState>,
    gpu: ResMut<GPUContext>,
) {
    if ui_state.ca_states != 0 && ui_state.ca_thresh != 0 {
        context.model.set_dynamic(Box::new(CyclicAutomaton::new(
            ui_state.ca_states,
            ui_state.ca_thresh,
        )));
    }

    // let r = tokio::runtime::Runtime::new().unwrap();
    // r.block_on(context.model.compute_sync_wgpu(&gpu.device));

    context.model.compute_sync();

    let image = images.get_mut(gpu.image_handler.as_ref().unwrap()).unwrap();
    let dynamic = context.model.dynamic() as &dyn LocalDynamic<DiscreteState, ()>;
    let states = dynamic.states();

    let data = context
        .model
        .space_state()
        .iter()
        .map(|x| colorous::TURBO.eval_continuous(x.state() as f64 / states as f64))
        .flat_map(|col| {
            [
                col.r, // ((v as u32 * 255) / states) as u8,
                col.g, // ((v as u32 * 255) / states) as u8,
                col.b, // ((v as u32 * 255) / states) as u8,
                255,
            ]
        })
        .collect::<Vec<u8>>();

    image.data = data;

    materials
        .get_mut(gpu.material_handler.as_ref().unwrap())
        .unwrap()
        .base_color_texture = Some(gpu.image_handler.as_ref().unwrap().clone());
}

fn setup(
    mut commands: Commands<'_, '_>,
    mut meshes: ResMut<'_, Assets<Mesh>>,
    mut materials: ResMut<'_, Assets<StandardMaterial>>,
    mut images: ResMut<'_, Assets<Image>>,
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

    // let runtime = tokio::runtime::Builder::new_current_thread()
    //     .build()
    //     .unwrap();

    // let device = runtime.block_on(create_gpu_device());

    commands.insert_resource(GPUContext {
        // device: Arc::new(device),
        material_handler: Some(material_handle.clone()),
        image_handler: Some(image_handle.clone()),
    });
}
