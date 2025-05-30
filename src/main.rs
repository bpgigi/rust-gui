use eframe::{run_native, App, CreationContext, NativeOptions};
use egui::{Context, FontData, FontDefinitions, FontFamily}; // Added Font types
use egui_graphs::{Graph, GraphView, DefaultNodeShape, DefaultEdgeShape, SettingsStyle};
use std::fs; // For reading the font file
use petgraph::stable_graph::{StableGraph, DefaultIx};
use petgraph::Directed;

pub struct BasicApp {
    g: Graph<String, ()>,
    style_labels_always: bool, // 新增：用于存储标签显示设置
}

impl BasicApp {
    fn new(cc: &CreationContext<'_>) -> Self { // Added cc parameter to access egui_ctx
        // --- FONT SETUP START ---
        let mut fonts = FontDefinitions::default();

        // Attempt to load "微软雅黑" from an "assets" folder
        // Attempt to load "simsun.ttc" directly from Windows system font directory.
        // Note: This approach is not portable to other operating systems.
        let font_path = "C:\\Windows\\Fonts\\simsun.ttc";
        match fs::read(font_path) {
            Ok(font_bytes) => {
                fonts.font_data.insert(
                    "my_chinese_font".to_owned(), // Give it a name
                    FontData::from_owned(font_bytes).into(), // Convert to Arc<FontData>
                );

                // Prioritize our new font for proportional (standard) text
                fonts.families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(0, "my_chinese_font".to_owned());

                // Optionally, also for monospace text
                fonts.families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .insert(0, "my_chinese_font".to_owned());
                
                println!("Successfully loaded font: {}", font_path);
            }
            Err(e) => {
                eprintln!("Error loading font file at '{}': {}. Chinese characters might not display correctly.", font_path, e);
                // Fallback: you could try to add system font names here if you know them and egui supports it,
                // but loading from file is more reliable.
                // For example, on some systems, egui might find "Microsoft YaHei" if installed.
                // fonts.families.entry(FontFamily::Proportional).or_default().insert(0, "Microsoft YaHei".to_owned());
            }
        }
        
        cc.egui_ctx.set_fonts(fonts);
        // --- FONT SETUP END ---

        let graph_data = generate_graph(); // This is petgraph::StableGraph<String, ()>
        let mut egui_graph = Graph::<String, (), Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape>::from(&graph_data);

        // Iterate over the nodes and set their labels explicitly from the payload
        for node_idx in graph_data.node_indices() {
            if let Some(payload_str) = graph_data.node_weight(node_idx) {
                if let Some(egui_node) = egui_graph.node_mut(node_idx) {
                    egui_node.set_label(payload_str.clone());
                }
            }
        }

        Self {
            g: egui_graph,
            style_labels_always: true, // 初始化设置
        }
    }
}

impl App for BasicApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        egui::SidePanel::right("config_panel")
            .min_width(200.0)
            .show(ctx, |ui| {
                ui.heading("配置面板");
                ui.separator();
                
                ui.collapsing("样式设置", |ui| {
                    ui.checkbox(&mut self.style_labels_always, "总是显示标签");
                });
                // 以后我们会在这里添加具体的UI控件
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            // 使用 self.style_labels_always 来动态配置
            let style_settings = SettingsStyle::new().with_labels_always(self.style_labels_always);
            let mut graph_view =
                GraphView::<String, (), Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape>::new(
                    &mut self.g,
                )
                .with_styles(&style_settings);
            ui.add(&mut graph_view);
        });
    }
}

fn generate_graph() -> StableGraph<String, ()> {
    let mut g = StableGraph::<String, ()>::new();

    let a = g.add_node("节点A".to_string());
    let b = g.add_node("节点B".to_string());
    let c = g.add_node("节点C".to_string());
    let d = g.add_node("节点D".to_string());
    let e = g.add_node("节点E".to_string());

    g.add_edge(a, b, ());
    g.add_edge(b, c, ());
    g.add_edge(c, a, ());
    g.add_edge(a, d, ());
    g.add_edge(d, e, ());
    g.add_edge(e, a, ());
    g.add_edge(b, e, ());

    g
}

fn main() {
    run_native(
        "egui_graphs_basic_standalone_demo",
        NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(BasicApp::new(cc)))),
    )
    .unwrap();
}