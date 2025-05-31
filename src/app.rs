use eframe::{App, CreationContext};
use egui::{Context, FontData, FontDefinitions, FontFamily}; // Removed ScrollArea
use egui_graphs::{Graph, events::Event}; // Removed GraphView, SettingsStyle, SettingsNavigation, SettingsInteraction
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use petgraph::stable_graph::{StableGraph, DefaultIx, NodeIndex, EdgeIndex};
use petgraph::{Directed, Undirected, EdgeType};
use rand::{Rng, rngs::ThreadRng};
use fdg::{ForceGraph, Force};
use fdg::fruchterman_reingold::FruchtermanReingold;
use crossbeam_channel::{unbounded, Sender, Receiver};

// Moved from main.rs
pub const DEFAULT_NODE_COUNT: usize = 15;
pub const DEFAULT_EDGE_COUNT: usize = 20;

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
    pub g: AppGraph,
    pub is_directed: bool,
    pub style_labels_always: bool,
    pub nav_fit_to_screen: bool,
    pub nav_zoom_and_pan: bool,
    pub nav_zoom_speed: f32,
    pub ia_dragging_enabled: bool,
    pub ia_node_clicking_enabled: bool,
    pub ia_node_selection_enabled: bool,
    pub ia_node_selection_multi_enabled: bool,
    pub ia_edge_clicking_enabled: bool,
    pub ia_edge_selection_enabled: bool,
    pub ia_edge_selection_multi_enabled: bool,

    pub sim: ForceGraph<f32, 2, NodePayload, EdgePayload>,
    pub force_algo: FruchtermanReingold<f32, 2>,
    
    pub sim_dt: f32,
    pub sim_cooloff_factor: f32,
    pub sim_scale: f32,
    pub simulation_stopped: bool,

    pub graph_nodes_count: usize,
    pub graph_edges_count: usize,
    pub rng: ThreadRng,
    pub event_publisher: Sender<Event>,
    pub event_consumer: Receiver<Event>,
    pub node_label_to_index_map: HashMap<String, NodeIndex<DefaultIx>>,

    // Fields for UI state that will be managed by settings_panel
    // These will be passed to the settings_panel drawing function
    // For now, they remain here to keep the app compiling during refactor.
    // Later, we might move their direct mutation logic to settings_panel if it makes sense,
    // or pass mutable references.
    pub input_node_from: String,
    pub input_node_to: String,
    pub input_node_to_add: String,
    pub input_node_to_remove: String,
}

impl BasicApp {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        let mut fonts = FontDefinitions::default();
        let font_path = "C:\\Windows\\Fonts\\simsun.ttc";
        match fs::read(font_path) {
            Ok(font_bytes) => {
                let mut loaded_font_data = FontData::from_owned(font_bytes);
                loaded_font_data.tweak.scale = 0.8;
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
        let rng = rand::rngs::ThreadRng::default();
        
        let initial_is_directed = true;
        let initial_petgraph = StableGraph::<NodePayload, EdgePayload, Directed>::new();
        let initial_egui_graph = Graph::<NodePayload, EdgePayload, Directed>::from(&initial_petgraph);

        let mut app = Self {
            g: AppGraph::Directed(initial_egui_graph),
            is_directed: initial_is_directed,
            style_labels_always: true,
            nav_fit_to_screen: false,
            nav_zoom_and_pan: true,
            nav_zoom_speed: 0.01,
            ia_dragging_enabled: true,
            ia_node_clicking_enabled: true,
            ia_node_selection_enabled: true,
            ia_node_selection_multi_enabled: true,
            ia_edge_clicking_enabled: true,
            ia_edge_selection_enabled: true,
            ia_edge_selection_multi_enabled: true,
            sim: fdg::ForceGraph::new(), 
            force_algo: fdg::fruchterman_reingold::FruchtermanReingold {
                conf: fdg::fruchterman_reingold::FruchtermanReingoldConfiguration { 
                    dt: 0.035, cooloff_factor: 0.95, scale: 100.0 
                },
                velocities: HashMap::default(),
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
            input_node_from: String::new(),
            input_node_to: String::new(),
            input_node_to_add: String::new(),
            input_node_to_remove: String::new(),
        };

        app.reset_graph_and_simulation();
        
        app
    }

    pub fn reset_graph_and_simulation(&mut self) {
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

        self.force_algo = fdg::fruchterman_reingold::FruchtermanReingold {
            conf: fdg::fruchterman_reingold::FruchtermanReingoldConfiguration {
                dt: self.sim_dt,
                cooloff_factor: self.sim_cooloff_factor,
                scale: self.sim_scale,
            },
            velocities: HashMap::default(),
        };
        
        for _ in 0..100 { Force::apply(&mut self.force_algo, &mut self.sim); }
        Self::sync_node_positions_to_egui(&self.sim, &mut self.g, &self.node_label_to_index_map);
    }

    fn populate_graph_data<Ty: EdgeType>(
        graph_data: &mut StableGraph<NodePayload, EdgePayload, Ty>,
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

    fn initialize_egui_node_positions<Ty: EdgeType>(
        egui_graph: &mut Graph<NodePayload, EdgePayload, Ty>,
        petgraph_graph: &StableGraph<NodePayload, EdgePayload, Ty>,
        rng: &mut ThreadRng,
    ) {
        for node_idx_pet in petgraph_graph.node_indices() {
            let egui_node_idx = node_idx_pet;

            if let Some(node_payload) = petgraph_graph.node_weight(node_idx_pet) {
                if let Some(egui_node) = egui_graph.node_mut(egui_node_idx) {
                    egui_node.set_label(node_payload.label.clone());
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
                    egui_edge.set_label(edge_payload.label.clone());
                }
            }
        }
    }

    fn sync_node_positions_to_egui(
        sim_g: &fdg::ForceGraph<f32, 2, NodePayload, EdgePayload>,
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
    
    fn sync_specific_graph<Ty: EdgeType>(
        sim_g: &fdg::ForceGraph<f32, 2, NodePayload, EdgePayload>,
        egui_g_specific: &mut Graph<NodePayload, EdgePayload, Ty>,
        node_label_to_index_map: &HashMap<String, NodeIndex<DefaultIx>>
    ) {
        for (node_payload_from_sim, sim_pos_point) in sim_g.node_weights() {
            if let Some(&node_idx_for_egui) = node_label_to_index_map.get(&node_payload_from_sim.label) {
                if let Some(node_widget_in_egui) = egui_g_specific.node_mut(node_idx_for_egui) {
                    node_widget_in_egui.set_location(eframe::egui::Pos2::new(sim_pos_point.coords.x, sim_pos_point.coords.y));
                }
            }
        }
    }

    pub fn update_simulation(&mut self) {
        if !self.simulation_stopped {
            Force::apply(&mut self.force_algo, &mut self.sim); 
        }
    }

    pub fn handle_events(&mut self) {
        while let Ok(event) = self.event_consumer.try_recv() {
            match event {
                Event::NodeMove(payload) => {
                    let node_idx = NodeIndex::new(payload.id);
                    if let Some(node_weight_tuple_in_sim) = self.sim.node_weight_mut(node_idx) {
                        node_weight_tuple_in_sim.1.coords.x = payload.new_pos[0];
                        node_weight_tuple_in_sim.1.coords.y = payload.new_pos[1];
                        self.force_algo.velocities.remove(&node_idx);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn add_node_ui(&mut self, label: String) {
        if label.is_empty() {
            println!("Node label cannot be empty.");
            return;
        }
        if self.node_label_to_index_map.contains_key(&label) {
            println!("Node with label '{}' already exists.", label);
            return;
        }

        let payload = NodePayload { label: label.clone(), weight: self.rng.random_range(1.0_f32..10.0_f32) };
        let new_node_idx: NodeIndex<DefaultIx>;

        // Add to egui_graphs Graph and fdg::ForceGraph
        match &mut self.g {
            AppGraph::Directed(g) => {
                new_node_idx = g.add_node(payload.clone());
                // Initialize position for the new node in egui_graph
                if let Some(node_mut) = g.node_mut(new_node_idx) {
                    let x = self.rng.random_range(-50.0..50.0); // Or some other logic
                    let y = self.rng.random_range(-50.0..50.0);
                    node_mut.set_location(eframe::egui::Pos2::new(x, y));
                    node_mut.set_label(label.clone());
                }
            }
            AppGraph::Undirected(g) => {
                new_node_idx = g.add_node(payload.clone());
                if let Some(node_mut) = g.node_mut(new_node_idx) {
                    let x = self.rng.random_range(-50.0..50.0);
                    let y = self.rng.random_range(-50.0..50.0);
                    node_mut.set_location(eframe::egui::Pos2::new(x, y));
                    node_mut.set_label(label.clone());
                }
            }
        }
        
        // Add to fdg simulation graph.
        // fdg::ForceGraph::add_node takes N (NodePayload) and Point.
        // We need to decide an initial position for the simulation as well.
        // For simplicity, let's use the same random position as for egui_graph.
        // However, fdg positions are relative to its own coordinate system, often centered at (0,0).
        // The scale factor in FruchtermanReingold also plays a role.
        // Let's use a small random offset for fdg as well.
        let sim_pos_x = self.rng.random_range(-1.0..1.0); // Small initial displacement for simulation
        let sim_pos_y = self.rng.random_range(-1.0..1.0);
        // fdg expects fdg::nalgebra::Point2<f32> or similar
        let sim_point = fdg::nalgebra::Point2::new(sim_pos_x, sim_pos_y);
        // fdg::ForceGraph::add_node returns its own NodeIndex.
        // We rely on labels for mapping between egui_graphs and fdg for now.
        // The new_node_idx from egui_graphs is what we store in node_label_to_index_map.
        let _fdg_node_idx = self.sim.add_node((payload, sim_point));


        self.node_label_to_index_map.insert(label.clone(), new_node_idx);
        self.graph_nodes_count += 1;
        println!("Node '{}' added with index {:?}.", label, new_node_idx);
    }
    
    pub fn remove_node_ui(&mut self, label: String) {
        if label.is_empty() {
            println!("Node label to remove cannot be empty.");
            return;
        }
        
        if let Some(&node_idx_to_remove) = self.node_label_to_index_map.get(&label) {
            // Remove from egui_graphs Graph
            let _ = match &mut self.g { // Explicitly ignore Option<NodePayload>
                AppGraph::Directed(g) => { g.remove_node(node_idx_to_remove).map(|_| ()); },
                AppGraph::Undirected(g) => { g.remove_node(node_idx_to_remove).map(|_| ()); },
            };
    
            // Remove from fdg::ForceGraph
            // fdg::ForceGraph::remove_node takes a NodeIndex and returns Option<(N, Point)>
            let _removed_node_fdg = self.sim.remove_node(node_idx_to_remove);
            if _removed_node_fdg.is_none() {
                println!("Warning: Node {:?} not found in fdg simulation or already removed.", node_idx_to_remove);
            }
    
            self.node_label_to_index_map.remove(&label);
            self.graph_nodes_count = self.node_label_to_index_map.len(); // Update count based on map
            
            // Also update edge count as edges connected to this node are removed automatically by petgraph
             self.graph_edges_count = match &self.g {
                AppGraph::Directed(g) => g.edge_count(),
                AppGraph::Undirected(g) => g.edge_count(),
            };

            println!("Node '{}' ({:?}) removed.", label, node_idx_to_remove);
        } else {
            println!("Node with label '{}' not found for removal.", label);
        }
    }


    pub fn add_edge_ui(&mut self, from_label: String, to_label: String) {
        if from_label.is_empty() || to_label.is_empty() {
            println!("Node labels for adding edge cannot be empty.");
            return;
        }
        if from_label == to_label {
            println!("Cannot add self-loop via UI with identical labels.");
            return;
        }

        let n1_idx_opt = self.node_label_to_index_map.get(&from_label).copied();
        let n2_idx_opt = self.node_label_to_index_map.get(&to_label).copied();

        if let (Some(n1_idx), Some(n2_idx)) = (n1_idx_opt, n2_idx_opt) {
            let edge_label = format!("边: {}->{}", from_label, to_label);
            let edge_payload = EdgePayload { label: edge_label, weight: self.rng.random_range(1.0_f32..5.0_f32) };

            let _ = match &mut self.g { // Ignore return value
                AppGraph::Directed(g) => { g.add_edge(n1_idx, n2_idx, edge_payload.clone()); },
                AppGraph::Undirected(g) => { g.add_edge(n1_idx, n2_idx, edge_payload.clone()); },
            };
            self.sim.add_edge(n1_idx, n2_idx, edge_payload);
            self.graph_edges_count = match &self.g {
                AppGraph::Directed(g) => g.edge_count(),
                AppGraph::Undirected(g) => g.edge_count(),
            };
            println!("Edge added between '{}' ({:?}) and '{}' ({:?})", from_label, n1_idx, to_label, n2_idx);
        } else {
            if n1_idx_opt.is_none() { println!("Node '{}' not found.", from_label); }
            if n2_idx_opt.is_none() { println!("Node '{}' not found.", to_label); }
        }
    }
    
    // This function is for the button "Add Edge Between Selected"
    pub fn add_edge_between_selected_nodes(&mut self) {
        let selected_node_indices: Vec<NodeIndex<DefaultIx>> = match &self.g {
            AppGraph::Directed(g) => g.selected_nodes().iter().copied().collect(),
            AppGraph::Undirected(g) => g.selected_nodes().iter().copied().collect(),
        };

        if selected_node_indices.len() == 2 {
            let n1_idx = selected_node_indices[0];
            let n2_idx = selected_node_indices[1];

            if n1_idx == n2_idx {
                println!("Cannot add self-loop.");
                return;
            }

            let n1_label = match &self.g {
                AppGraph::Directed(g) => g.node(n1_idx).map_or_else(|| "N/A".to_string(), |n| n.payload().label.clone()),
                AppGraph::Undirected(g) => g.node(n1_idx).map_or_else(|| "N/A".to_string(), |n| n.payload().label.clone()),
            };
            let n2_label = match &self.g {
                AppGraph::Directed(g) => g.node(n2_idx).map_or_else(|| "N/A".to_string(), |n| n.payload().label.clone()),
                AppGraph::Undirected(g) => g.node(n2_idx).map_or_else(|| "N/A".to_string(), |n| n.payload().label.clone()),
            };

            let edge_label = format!("边: {}->{}", n1_label, n2_label);
            let edge_payload = EdgePayload { label: edge_label, weight: 1.0 };

            let _ = match &mut self.g {
                AppGraph::Directed(g) => { g.add_edge(n1_idx, n2_idx, edge_payload.clone()); },
                AppGraph::Undirected(g) => { g.add_edge(n1_idx, n2_idx, edge_payload.clone()); },
            };
            self.sim.add_edge(n1_idx, n2_idx, edge_payload);
            self.graph_edges_count = match &self.g {
                AppGraph::Directed(g) => g.edge_count(),
                AppGraph::Undirected(g) => g.edge_count(),
            };
            println!("Edge added between selected {:?} and {:?}", n1_idx, n2_idx);
        } else {
            println!("Please select exactly two nodes to add an edge.");
        }
    }

    pub fn remove_selected_edges_ui(&mut self) {
        let selected_edge_indices: Vec<EdgeIndex<DefaultIx>> = match &self.g {
            AppGraph::Directed(g) => g.selected_edges().iter().copied().collect(),
            AppGraph::Undirected(g) => g.selected_edges().iter().copied().collect(),
        };

        if selected_edge_indices.is_empty() {
            println!("No edges selected to remove.");
            return;
        }

        for edge_idx in selected_edge_indices {
            match &mut self.g {
                AppGraph::Directed(g) => { g.remove_edge(edge_idx); },
                AppGraph::Undirected(g) => { g.remove_edge(edge_idx); },
            };
            let _removed_edge_payload_fdg = self.sim.remove_edge(edge_idx);
            if _removed_edge_payload_fdg.is_none() {
                println!("Warning: Edge {:?} not found in fdg simulation or already removed.", edge_idx);
            }
            println!("Edge {:?} removed.", edge_idx);
        }

        self.graph_edges_count = match &self.g {
            AppGraph::Directed(g) => g.edge_count(),
            AppGraph::Undirected(g) => g.edge_count(),
        };
        match &mut self.g {
            AppGraph::Directed(g) => { g.set_selected_edges(Default::default()); }
            AppGraph::Undirected(g) => { g.set_selected_edges(Default::default()); }
        };
    }
}

// The impl App for BasicApp will be moved here as well,
// but its update method will call out to settings_panel::draw_settings_panel
// and graph_view::draw_graph_view (or similar).
// For now, the full update method is kept here and will be split later.
impl App for BasicApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // Event handling should happen early
        self.handle_events();
        
        // Simulation update
        self.update_simulation();
        Self::sync_node_positions_to_egui(&self.sim, &mut self.g, &self.node_label_to_index_map);

        // Draw settings panel (this will be moved to settings_panel.rs)
        crate::settings_panel::draw_settings_panel(self, ctx);
        
        // Draw graph view (this will be moved to graph_view.rs)
        crate::graph_view::draw_graph_view(self, ctx, frame);
    }
}