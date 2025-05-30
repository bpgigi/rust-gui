use eframe::{run_native, App, CreationContext};
use egui::{Context, FontData, FontDefinitions, FontFamily, ScrollArea};
use egui_graphs::{Graph, GraphView, DefaultNodeShape, DefaultEdgeShape, SettingsStyle, SettingsNavigation, SettingsInteraction, events::Event};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc; // Ensure Arc is imported
use petgraph::stable_graph::{StableGraph, DefaultIx, NodeIndex};
use petgraph::{Directed, Undirected, EdgeType}; // Added Undirected and EdgeType
use rand::{Rng, rngs::ThreadRng}; // Per user suggestion
use fdg::{ForceGraph, Force}; // Per user suggestion
use fdg::fruchterman_reingold::FruchtermanReingold;
// use fdg::nalgebra as na; // Removed as nalgebra types are accessed via fdg::nalgebra or not directly needed
use crossbeam_channel::{unbounded, Sender, Receiver};

const DEFAULT_NODE_COUNT: usize = 15;
const DEFAULT_EDGE_COUNT: usize = 20;

// Enum to hold either a directed or an undirected graph
#[derive(Clone, Debug, Default)]
pub struct NodePayload {
    pub label: String,
    pub weight: f32,
}

#[derive(Clone, Debug, Default)]
pub struct EdgePayload {
    pub label: String,
    pub weight: f32,
}

pub enum AppGraph {
    Directed(Graph<NodePayload, EdgePayload, Directed>),
    Undirected(Graph<NodePayload, EdgePayload, Undirected>),
}

pub struct BasicApp {
    g: AppGraph,
    is_directed: bool, // To track current graph type
    style_labels_always: bool,
    nav_fit_to_screen: bool,
    nav_zoom_and_pan: bool,
    nav_zoom_speed: f32,
    ia_dragging_enabled: bool,
    ia_node_clicking_enabled: bool,
    ia_node_selection_enabled: bool,
    ia_node_selection_multi_enabled: bool,
    ia_edge_clicking_enabled: bool,
    ia_edge_selection_enabled: bool,
    ia_edge_selection_multi_enabled: bool,

    sim: ForceGraph<f32, 2, NodePayload, EdgePayload>, // Updated for fdg
    force_algo: FruchtermanReingold<f32, 2>,
    
    sim_dt: f32,
    sim_cooloff_factor: f32,
    sim_scale: f32,
    simulation_stopped: bool,

    graph_nodes_count: usize,
    graph_edges_count: usize,
    rng: ThreadRng,
    event_publisher: Sender<Event>,
    event_consumer: Receiver<Event>,
    node_label_to_index_map: HashMap<String, NodeIndex<DefaultIx>>,
}

impl BasicApp {
    fn new(cc: &CreationContext<'_>) -> Self {
        let mut fonts = FontDefinitions::default();
        let font_path = "C:\\Windows\\Fonts\\simsun.ttc"; // Or msyh.ttf if preferred and available
        match fs::read(font_path) {
            Ok(font_bytes) => {
                let mut loaded_font_data = FontData::from_owned(font_bytes);
                loaded_font_data.tweak.scale = 0.8; // Adjust font scale
                fonts.font_data.insert("my_chinese_font".to_owned(), Arc::new(loaded_font_data));
                fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "my_chinese_font".to_owned());
                fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "my_chinese_font".to_owned());
                println!("Successfully loaded font: {}", font_path);
            }
            Err(e) => {
                eprintln!("Error loading font file at '{}': {}. Chinese characters might not display correctly.", font_path, e);
            }
        }
        cc.egui_ctx.set_fonts(fonts);

        let (event_publisher, event_consumer) = unbounded();
        let rng = rand::rngs::ThreadRng::default(); // Per user suggestion, updated to new API
        
        let initial_is_directed = true;
        // Placeholder types updated to NodePayload, EdgePayload
        let initial_petgraph = StableGraph::<NodePayload, EdgePayload, Directed>::new();
        let initial_egui_graph = Graph::<NodePayload, EdgePayload, Directed>::from(&initial_petgraph);

        let mut app = Self {
            g: AppGraph::Directed(initial_egui_graph),
            is_directed: initial_is_directed,
            style_labels_always: true,
            nav_fit_to_screen: true,
            nav_zoom_and_pan: false,
            nav_zoom_speed: 0.05,
            ia_dragging_enabled: true,
            ia_node_clicking_enabled: true,
            ia_node_selection_enabled: true,
            ia_node_selection_multi_enabled: false,
            ia_edge_clicking_enabled: false,
            ia_edge_selection_enabled: false,
            ia_edge_selection_multi_enabled: false,
            sim: fdg::ForceGraph::new(), 
            force_algo: fdg::fruchterman_reingold::FruchtermanReingold { // Per user suggestion
                conf: fdg::fruchterman_reingold::FruchtermanReingoldConfiguration { 
                    dt: 0.035, cooloff_factor: 0.95, scale: 100.0 
                },
                velocities: HashMap::default(), // Per user suggestion
            },
            sim_dt: 0.035,
            sim_cooloff_factor: 0.95,
            sim_scale: 100.0,
            simulation_stopped: false,
            graph_nodes_count: DEFAULT_NODE_COUNT,
            graph_edges_count: DEFAULT_EDGE_COUNT,
            rng,
            event_publisher,
            event_consumer,
            node_label_to_index_map: HashMap::new(),
        };

        app.reset_graph_and_simulation();
        
        app
    }

    fn reset_graph_and_simulation(&mut self) {
        self.node_label_to_index_map.clear();
        let petgraph_graph_for_fdg: StableGraph<NodePayload, EdgePayload, Directed>;

        if self.is_directed {
            let mut pet_graph_directed = StableGraph::<NodePayload, EdgePayload, Directed>::new();
            Self::populate_graph_data(&mut pet_graph_directed, self.graph_nodes_count, self.graph_edges_count, &mut self.rng, &mut self.node_label_to_index_map);
            
            let mut egui_graph = Graph::<NodePayload, EdgePayload, Directed>::from(&pet_graph_directed);
            Self::initialize_egui_node_positions(&mut egui_graph, &pet_graph_directed, &mut self.rng);

            self.g = AppGraph::Directed(egui_graph);
            petgraph_graph_for_fdg = pet_graph_directed;
        } else {
            let mut pet_graph_undirected = StableGraph::<NodePayload, EdgePayload, Undirected>::default();
            Self::populate_graph_data(&mut pet_graph_undirected, self.graph_nodes_count, self.graph_edges_count, &mut self.rng, &mut self.node_label_to_index_map);

            let mut egui_graph = Graph::<NodePayload, EdgePayload, Undirected>::from(&pet_graph_undirected);
            Self::initialize_egui_node_positions(&mut egui_graph, &pet_graph_undirected, &mut self.rng);
            self.g = AppGraph::Undirected(egui_graph);

            let mut directed_temp_graph = StableGraph::<NodePayload, EdgePayload, Directed>::new();
            for node_idx in pet_graph_undirected.node_indices() {
                if let Some(payload) = pet_graph_undirected.node_weight(node_idx) {
                    directed_temp_graph.add_node(payload.clone());
                }
            }
            for edge_idx in pet_graph_undirected.edge_indices() {
                if let (Some(source), Some(target)) = (pet_graph_undirected.edge_endpoints(edge_idx).map(|(s, _)| s), pet_graph_undirected.edge_endpoints(edge_idx).map(|(_, t)| t)) {
                    if let Some(payload) = pet_graph_undirected.edge_weight(edge_idx) {
                        directed_temp_graph.add_edge(source, target, payload.clone());
                    }
                }
            }
            petgraph_graph_for_fdg = directed_temp_graph;
        }
        
        self.sim = fdg::init_force_graph_uniform(petgraph_graph_for_fdg, 100.0);

        self.force_algo = fdg::fruchterman_reingold::FruchtermanReingold { // Per user suggestion
            conf: fdg::fruchterman_reingold::FruchtermanReingoldConfiguration {
                dt: self.sim_dt,
                cooloff_factor: self.sim_cooloff_factor,
                scale: self.sim_scale,
            },
            velocities: HashMap::default(), // Per user suggestion
        };
        
        for _ in 0..100 { Force::apply(&mut self.force_algo, &mut self.sim); }
        Self::sync_node_positions_to_egui(&self.sim, &mut self.g, &self.node_label_to_index_map);
    }

    // Helper to populate petgraph data
    fn populate_graph_data<Ty: EdgeType>(
        graph_data: &mut StableGraph<NodePayload, EdgePayload, Ty>, // Updated type
        node_count: usize,
        edge_count: usize,
        rng: &mut ThreadRng,
        node_label_to_index_map: &mut HashMap<String, NodeIndex<DefaultIx>>,
    ) {
        for i in 0..node_count {
            let label_str = format!("节点{}", i);
            let payload = NodePayload { label: label_str.clone(), weight: rng.random_range(1.0_f32..10.0_f32) };
            let node_idx = graph_data.add_node(payload);
            node_label_to_index_map.insert(label_str, node_idx);
        }

        if node_count > 0 {
            for _ in 0..edge_count {
                let source_idx_val = rng.random_range(0..node_count);
                let target_idx_val = rng.random_range(0..node_count);
                if source_idx_val != target_idx_val {
                    let source_node_index = NodeIndex::new(source_idx_val);
                    let target_node_index = NodeIndex::new(target_idx_val);
                    
                    if graph_data.node_weight(source_node_index).is_some() && graph_data.node_weight(target_node_index).is_some() {
                        let edge_label_str = format!("边 {}-{}", source_idx_val, target_idx_val);
                        let edge_payload = EdgePayload { label: edge_label_str, weight: rng.random_range(1.0_f32..5.0_f32) };
                        graph_data.add_edge(source_node_index, target_node_index, edge_payload);
                    }
                }
            }
        }
    }

    // Helper to initialize egui_graphs node positions and labels from petgraph
    fn initialize_egui_node_positions<Ty: EdgeType>(
        egui_graph: &mut Graph<NodePayload, EdgePayload, Ty>, // Updated type
        petgraph_graph: &StableGraph<NodePayload, EdgePayload, Ty>, // Updated type
        rng: &mut ThreadRng,
    ) {
        for node_idx_pet in petgraph_graph.node_indices() {
            let egui_node_idx = node_idx_pet;

            if let Some(node_payload) = petgraph_graph.node_weight(node_idx_pet) {
                if let Some(egui_node) = egui_graph.node_mut(egui_node_idx) {
                    egui_node.set_label(node_payload.label.clone()); // Use label from NodePayload
                    let x = rng.random_range(-200.0..200.0);
                    let y = rng.random_range(-200.0..200.0);
                    egui_node.set_location(eframe::egui::Pos2::new(x,y));
                }
            }
        }
        for edge_idx_pet in petgraph_graph.edge_indices() {
            let egui_edge_idx = edge_idx_pet;
            if let Some(edge_payload) = petgraph_graph.edge_weight(edge_idx_pet) {
                if let Some(egui_edge) = egui_graph.edge_mut(egui_edge_idx) {
                    egui_edge.set_label(edge_payload.label.clone()); // Use label from EdgePayload
                }
            }
        }
    }

    // Renamed and signature changed to accept AppGraph
    fn sync_node_positions_to_egui(
        sim_g: &fdg::ForceGraph<f32, 2, NodePayload, EdgePayload>, // Updated N, E types for sim_g
        app_g: &mut AppGraph,
        node_label_to_index_map: &HashMap<String, NodeIndex<DefaultIx>>
    ) {
        match app_g {
            AppGraph::Directed(g_directed) => {
                Self::sync_specific_graph(sim_g, g_directed, node_label_to_index_map);
            }
            AppGraph::Undirected(g_undirected) => {
                Self::sync_specific_graph(sim_g, g_undirected, node_label_to_index_map);
            }
        }
    }
    
    // Helper for sync_node_positions_to_egui
    fn sync_specific_graph<Ty: EdgeType>(
        sim_g: &fdg::ForceGraph<f32, 2, NodePayload, EdgePayload>, // Updated sim_g type
        egui_g_specific: &mut Graph<NodePayload, EdgePayload, Ty>, // Updated egui_g_specific type
        node_label_to_index_map: &HashMap<String, NodeIndex<DefaultIx>>
    ) {
        // Assuming sim_g.node_weights() for this fdg version returns (NodePayload, fdg::nalgebra::OPoint)
        for (node_payload_from_sim, sim_pos_point) in sim_g.node_weights() {
            // node_payload_from_sim: NodePayload
            // sim_pos_point: fdg::nalgebra::OPoint<f32, fdg::nalgebra::Const<2>>
            
            if let Some(&node_idx_for_egui) = node_label_to_index_map.get(&node_payload_from_sim.label) { // Use label from NodePayload
                if let Some(node_widget_in_egui) = egui_g_specific.node_mut(node_idx_for_egui) {
                    node_widget_in_egui.set_location(eframe::egui::Pos2::new(sim_pos_point.coords.x, sim_pos_point.coords.y));
                }
            }
        }
    }
    
    // Original sync_node_positions logic, to be removed or adapted if the above works
    // fn sync_node_positions(egui_g: &mut Graph<String, String>, sim_g: &fdg::ForceGraph<f32, 2, String, String>) {
    //     let indices: Vec<_> = egui_g.g.node_indices().collect();
    //     for idx in indices {
    //         if let Some(node_widget) = egui_g.node_mut(idx) {
    //              if let Some((_data, sim_node_pos_point)) = sim_g.node_weight(idx) {
    //                 node_widget.set_location(eframe::egui::Pos2::new(sim_node_pos_point.x, sim_node_pos_point.y));
    //             } // End of inner if, part of commented old function
    //         } // End of outer if, part of commented old function
    //     } // End of for loop, part of commented old function
    // } // End of commented old function

    fn update_simulation(&mut self) {
        if !self.simulation_stopped {
            Force::apply(&mut self.force_algo, &mut self.sim); 
        }
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.event_consumer.try_recv() {
            match event {
                Event::NodeMove(payload) => {
                    let node_idx = NodeIndex::new(payload.id);
                    // self.sim.node_weight_mut(node_idx) now returns Option<&mut (NodePayload, Point)>
                    if let Some(node_weight_tuple_in_sim) = self.sim.node_weight_mut(node_idx) {
                        // node_weight_tuple_in_sim is &mut (NodePayload, Point)
                        // We need to modify the Point, which is the second element of the tuple.
                        node_weight_tuple_in_sim.1.coords.x = payload.new_pos[0];
                        node_weight_tuple_in_sim.1.coords.y = payload.new_pos[1];
                        self.force_algo.velocities.remove(&node_idx);
                    }
                }
                _ => {}
            }
        }
    }
}

impl App for BasicApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        egui::SidePanel::right("config_panel")
            .min_width(250.0)
            .show(ctx, |ui| {
                ui.heading("配置面板");
                ui.separator();
                
                ScrollArea::vertical().show(ui, |ui| {
                    ui.collapsing("图属性", |ui| {
                        if ui.checkbox(&mut self.is_directed, "有向图").changed() {
                            self.reset_graph_and_simulation();
                        }
                    });
                    ui.separator();

                    ui.collapsing("样式设置", |ui| {
                        ui.checkbox(&mut self.style_labels_always, "总是显示标签");
                    });

                    ui.separator();

                    ui.collapsing("导航设置", |ui| {
                        if ui.checkbox(&mut self.nav_fit_to_screen, "适应屏幕").changed() {
                            if self.nav_fit_to_screen {
                                self.nav_zoom_and_pan = false;
                            }
                        }
                        ui.label("启用后，图表将始终缩放以适应屏幕。");
                        ui.add_space(5.0);
                        ui.add_enabled_ui(!self.nav_fit_to_screen, |ui| {
                            ui.checkbox(&mut self.nav_zoom_and_pan, "缩放与平移");
                            ui.label("启用后，可用Ctrl+滚轮缩放，鼠标中键拖拽平移。");
                            ui.add(egui::Slider::new(&mut self.nav_zoom_speed, 0.01..=0.5).text("缩放速度"));
                        });
                    });

                    ui.separator();

                    ui.collapsing("交互设置", |ui| {
                        if ui.checkbox(&mut self.ia_dragging_enabled, "节点可拖拽").on_hover_text("启用后，可按住鼠标左键拖拽节点。").changed() {
                            if self.ia_dragging_enabled { self.ia_node_clicking_enabled = true; }
                        }
                        ui.add_space(5.0);
                        ui.add_enabled_ui(!(self.ia_dragging_enabled || self.ia_node_selection_enabled || self.ia_node_selection_multi_enabled), |ui| {
                            ui.checkbox(&mut self.ia_node_clicking_enabled, "节点可点击").on_hover_text("启用后，可捕获节点点击事件。");
                        }).response.on_disabled_hover_text("节点拖拽或选择已启用时，点击自动启用");
                        ui.add_space(5.0);
                        ui.add_enabled_ui(!self.ia_node_selection_multi_enabled, |ui| {
                            if ui.checkbox(&mut self.ia_node_selection_enabled, "节点可选择").on_hover_text("启用后，可单击选择/取消选择节点。").changed() {
                                 if self.ia_node_selection_enabled { self.ia_node_clicking_enabled = true; }
                            }
                        }).response.on_disabled_hover_text("节点多选已启用时，单选自动启用");
                        ui.add_space(5.0);
                        if ui.checkbox(&mut self.ia_node_selection_multi_enabled, "节点可多选").on_hover_text("启用后，可按住Ctrl点击多选节点。").changed() {
                            if self.ia_node_selection_multi_enabled {
                                self.ia_node_clicking_enabled = true;
                                self.ia_node_selection_enabled = true;
                            }
                        }
                        ui.add_space(10.0);
                        ui.add_enabled_ui(!(self.ia_edge_selection_enabled || self.ia_edge_selection_multi_enabled), |ui| {
                             ui.checkbox(&mut self.ia_edge_clicking_enabled, "边可点击").on_hover_text("启用后，可捕获边点击事件。");
                        }).response.on_disabled_hover_text("边选择已启用时，点击自动启用");
                        ui.add_space(5.0);
                        ui.add_enabled_ui(!self.ia_edge_selection_multi_enabled, |ui| {
                            if ui.checkbox(&mut self.ia_edge_selection_enabled, "边可选择").on_hover_text("启用后，可单击选择/取消选择边。").changed() {
                                if self.ia_edge_selection_enabled { self.ia_edge_clicking_enabled = true; }
                            }
                        }).response.on_disabled_hover_text("边多选已启用时，单选自动启用");
                        ui.add_space(5.0);
                        if ui.checkbox(&mut self.ia_edge_selection_multi_enabled, "边可多选").on_hover_text("启用后，可按住Ctrl点击多选边。").changed() {
                            if self.ia_edge_selection_multi_enabled {
                                self.ia_edge_clicking_enabled = true;
                                self.ia_edge_selection_enabled = true;
                            }
                        }
                    });

                    ui.separator();

                    ui.collapsing("模拟控制", |ui| {
                        if ui.button(if self.simulation_stopped { "启动模拟" } else { "停止模拟" }).clicked() {
                            self.simulation_stopped = !self.simulation_stopped;
                        }
                        if ui.button("重置图与模拟").clicked() {
                            self.reset_graph_and_simulation();
                        }
    
                        ui.add_space(10.0);
                        ui.label("模拟参数:");
                        let mut reconfigure_force = false;
                        if ui.add(egui::Slider::new(&mut self.sim_dt, 0.001..=0.1).text("时间步长 (dt)")).changed() { reconfigure_force = true; }
                        if ui.add(egui::Slider::new(&mut self.sim_cooloff_factor, 0.1..=1.0).text("冷却因子")).changed() { reconfigure_force = true; }
                        if ui.add(egui::Slider::new(&mut self.sim_scale, 10.0..=500.0).text("缩放尺度")).changed() { reconfigure_force = true; }
                        if reconfigure_force {
                            self.force_algo.conf = fdg::fruchterman_reingold::FruchtermanReingoldConfiguration {
                                dt: self.sim_dt,
                                cooloff_factor: self.sim_cooloff_factor,
                                scale: self.sim_scale,
                            };
                        }
                        
                        ui.add_space(10.0);
                        ui.label("图生成:");
                        let mut re_generate = false;
                        if ui.add(egui::Slider::new(&mut self.graph_nodes_count, 1..=100).text("节点数量")).changed() {
                            re_generate = true;
                        }
                        if ui.add(egui::Slider::new(&mut self.graph_edges_count, 0..=200).text("边数量")).changed() {
                            re_generate = true;
                        }
                        if re_generate {
                           self.reset_graph_and_simulation();
                        }
                    });
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let style_settings = SettingsStyle::new().with_labels_always(self.style_labels_always);
            let nav_settings = SettingsNavigation::new()
                .with_fit_to_screen_enabled(self.nav_fit_to_screen)
                .with_zoom_and_pan_enabled(self.nav_zoom_and_pan)
                .with_zoom_speed(self.nav_zoom_speed);
            let interaction_settings = SettingsInteraction::new()
                .with_dragging_enabled(self.ia_dragging_enabled)
                .with_node_clicking_enabled(self.ia_node_clicking_enabled)
                .with_node_selection_enabled(self.ia_node_selection_enabled)
                .with_node_selection_multi_enabled(self.ia_node_selection_multi_enabled)
                .with_edge_clicking_enabled(self.ia_edge_clicking_enabled)
                .with_edge_selection_enabled(self.ia_edge_selection_enabled)
                .with_edge_selection_multi_enabled(self.ia_edge_selection_multi_enabled);

            match &mut self.g {
                AppGraph::Directed(g_directed) => {
                    let mut graph_view =
                        GraphView::<NodePayload, EdgePayload, Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape>::new(g_directed)
                            .with_styles(&style_settings)
                            .with_navigations(&nav_settings)
                            .with_interactions(&interaction_settings)
                            .with_events(&self.event_publisher);
                    ui.add(&mut graph_view);
                }
                AppGraph::Undirected(g_undirected) => {
                    let mut graph_view =
                        GraphView::<NodePayload, EdgePayload, Undirected, DefaultIx, DefaultNodeShape, DefaultEdgeShape>::new(g_undirected)
                            .with_styles(&style_settings)
                            .with_navigations(&nav_settings)
                            .with_interactions(&interaction_settings)
                            .with_events(&self.event_publisher);
                    ui.add(&mut graph_view);
                }
            }
        });

        self.handle_events();
        self.update_simulation();
        // The sync_node_positions_to_egui call was already updated in a previous step.
        // Original line before that was: Self::sync_node_positions(&mut self.g, &self.sim);
        // Current correct line (already in file from previous diff):
        Self::sync_node_positions_to_egui(&self.sim, &mut self.g, &self.node_label_to_index_map);
    }
}

fn main() {
    run_native(
        "egui_graphs_basic_standalone_demo",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(BasicApp::new(cc)))),
    )
    .unwrap();
}