use eframe::{run_native, App, CreationContext, NativeOptions};
use egui::{Context, FontData, FontDefinitions, FontFamily};
use egui_graphs::{Graph, GraphView, DefaultNodeShape, DefaultEdgeShape, SettingsStyle, SettingsNavigation, SettingsInteraction}; // Added SettingsInteraction
use std::fs;
use std::sync::Arc;
use petgraph::stable_graph::{StableGraph, DefaultIx};
use petgraph::Directed;

pub struct BasicApp {
    g: Graph<String, String>,
    style_labels_always: bool,
    // 导航设置状态
    nav_fit_to_screen: bool,
    nav_zoom_and_pan: bool,
    nav_zoom_speed: f32, // 新增：缩放速度控制
    // 交互设置状态
    ia_dragging_enabled: bool,
    ia_node_clicking_enabled: bool,
    ia_node_selection_enabled: bool,
    ia_node_selection_multi_enabled: bool,
    ia_edge_clicking_enabled: bool,
    ia_edge_selection_enabled: bool,
    ia_edge_selection_multi_enabled: bool,
}

impl BasicApp {
    fn new(cc: &CreationContext<'_>) -> Self {
        // --- FONT SETUP START ---
        let mut fonts = FontDefinitions::default();

        // Attempt to load "微软雅黑" from an "assets" folder
        // Attempt to load "simsun.ttc" directly from Windows system font directory.
        // Note: This approach is not portable to other operating systems.
        let font_path = "C:\\Windows\\Fonts\\simsun.ttc";
        match fs::read(font_path) {
            Ok(font_bytes) => {
                let mut loaded_font_data = FontData::from_owned(font_bytes);
                // 调整字体大小，例如缩小到80%
                loaded_font_data.tweak.scale = 0.8;
                
                fonts.font_data.insert(
                    "my_chinese_font".to_owned(), // Give it a name
                    Arc::new(loaded_font_data), // Manually wrap in Arc after tweaking
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

        let graph_data = generate_graph(); // 现在返回 petgraph::StableGraph<String, String>
        let mut egui_graph = Graph::<String, String, Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape>::from(&graph_data);

        // 设置节点标签
        for node_idx in graph_data.node_indices() {
            if let Some(payload_str) = graph_data.node_weight(node_idx) {
                if let Some(egui_node) = egui_graph.node_mut(node_idx) {
                    egui_node.set_label(payload_str.clone());
                }
            }
        }

        // 设置边标签
        for edge_idx in graph_data.edge_indices() {
            if let Some(payload_str) = graph_data.edge_weight(edge_idx) {
                if let Some(egui_edge) = egui_graph.edge_mut(edge_idx) {
                    egui_edge.set_label(payload_str.clone());
                }
            }
        }

        Self {
            g: egui_graph,
            style_labels_always: true,
            nav_fit_to_screen: true,
            nav_zoom_and_pan: false,
            nav_zoom_speed: 0.05, // 初始化缩放速度
            // 初始化交互设置 (默认为 false)
            ia_dragging_enabled: false,
            ia_node_clicking_enabled: false, // 会被其他选项自动启用
            ia_node_selection_enabled: false,
            ia_node_selection_multi_enabled: false,
            ia_edge_clicking_enabled: false, // 会被其他选项自动启用
            ia_edge_selection_enabled: false,
            ia_edge_selection_multi_enabled: false,
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

                ui.separator();

                ui.collapsing("导航设置", |ui| {
                    // "适应屏幕" 复选框
                    if ui.checkbox(&mut self.nav_fit_to_screen, "适应屏幕").changed() {
                        if self.nav_fit_to_screen {
                            // 如果启用了“适应屏幕”，则禁用“缩放与平移”
                            self.nav_zoom_and_pan = false;
                        }
                    }
                    ui.label("启用后，图表将始终缩放以适应屏幕。");

                    ui.add_space(5.0);

                    // "缩放与平移" 复选框，仅在“适应屏幕”未启用时可用
                    ui.add_enabled_ui(!self.nav_fit_to_screen, |ui| {
                        ui.checkbox(&mut self.nav_zoom_and_pan, "缩放与平移");
                        ui.label("启用后，可用Ctrl+滚轮缩放，鼠标中键拖拽平移。");
                        // 添加缩放速度滑块
                        ui.add(egui::Slider::new(&mut self.nav_zoom_speed, 0.01..=0.5).text("缩放速度"));
                    });
                });

                ui.separator();

                ui.collapsing("交互设置", |ui| {
                    // 节点交互
                    if ui.checkbox(&mut self.ia_dragging_enabled, "节点可拖拽").changed() && self.ia_dragging_enabled {
                        self.ia_node_clicking_enabled = true; // 拖拽需要节点可点击
                    }
                    ui.label("启用后，可按住鼠标左键拖拽节点。");
                    ui.add_space(5.0);

                    // 节点点击（如果其他节点交互未启用，则可以单独控制）
                    ui.add_enabled_ui(!(self.ia_dragging_enabled || self.ia_node_selection_enabled || self.ia_node_selection_multi_enabled), |ui| {
                        ui.checkbox(&mut self.ia_node_clicking_enabled, "节点可点击");
                    }).response.on_disabled_hover_text("节点拖拽或选择已启用时，点击自动启用");
                    ui.label("启用后，可捕获节点点击事件。");
                    ui.add_space(5.0);
                    
                    // 节点选择
                    ui.add_enabled_ui(!self.ia_node_selection_multi_enabled, |ui| {
                        if ui.checkbox(&mut self.ia_node_selection_enabled, "节点可选择").changed() && self.ia_node_selection_enabled {
                             self.ia_node_clicking_enabled = true; // 选择需要节点可点击
                        }
                    }).response.on_disabled_hover_text("节点多选已启用时，单选自动启用");
                    ui.label("启用后，可单击选择/取消选择节点。");
                    ui.add_space(5.0);

                    if ui.checkbox(&mut self.ia_node_selection_multi_enabled, "节点可多选").changed() && self.ia_node_selection_multi_enabled {
                        self.ia_node_clicking_enabled = true; // 多选需要节点可点击
                        self.ia_node_selection_enabled = true; // 多选时单选也应启用
                    }
                    ui.label("启用后，可按住Ctrl点击多选节点。");
                    ui.add_space(10.0);

                    // 边交互 (与节点类似)
                    ui.add_enabled_ui(!(self.ia_edge_selection_enabled || self.ia_edge_selection_multi_enabled), |ui| {
                         ui.checkbox(&mut self.ia_edge_clicking_enabled, "边可点击");
                    }).response.on_disabled_hover_text("边选择已启用时，点击自动启用");
                    ui.label("启用后，可捕获边点击事件。");
                    ui.add_space(5.0);

                    ui.add_enabled_ui(!self.ia_edge_selection_multi_enabled, |ui| {
                        if ui.checkbox(&mut self.ia_edge_selection_enabled, "边可选择").changed() && self.ia_edge_selection_enabled {
                            self.ia_edge_clicking_enabled = true;
                        }
                    }).response.on_disabled_hover_text("边多选已启用时，单选自动启用");
                    ui.label("启用后，可单击选择/取消选择边。");
                    ui.add_space(5.0);

                    if ui.checkbox(&mut self.ia_edge_selection_multi_enabled, "边可多选").changed() && self.ia_edge_selection_multi_enabled {
                        self.ia_edge_clicking_enabled = true;
                        self.ia_edge_selection_enabled = true;
                    }
                    ui.label("启用后，可按住Ctrl点击多选边。");
                });
                // 以后我们会在这里添加具体的UI控件
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let style_settings = SettingsStyle::new().with_labels_always(self.style_labels_always);
            let nav_settings = SettingsNavigation::new()
                .with_fit_to_screen_enabled(self.nav_fit_to_screen)
                .with_zoom_and_pan_enabled(self.nav_zoom_and_pan)
                .with_zoom_speed(self.nav_zoom_speed); // 应用缩放速度设置
            // 根据 BasicApp 中的状态构建 SettingsInteraction
            let interaction_settings = SettingsInteraction::new()
                .with_dragging_enabled(self.ia_dragging_enabled)
                .with_node_clicking_enabled(self.ia_node_clicking_enabled)
                .with_node_selection_enabled(self.ia_node_selection_enabled)
                .with_node_selection_multi_enabled(self.ia_node_selection_multi_enabled)
                .with_edge_clicking_enabled(self.ia_edge_clicking_enabled)
                .with_edge_selection_enabled(self.ia_edge_selection_enabled)
                .with_edge_selection_multi_enabled(self.ia_edge_selection_multi_enabled);

            let mut graph_view =
                GraphView::<String, String, Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape>::new(
                    &mut self.g,
                )
                .with_styles(&style_settings)
                .with_navigations(&nav_settings)
                .with_interactions(&interaction_settings); // 应用交互设置
            ui.add(&mut graph_view);
        });
    }
}

fn generate_graph() -> StableGraph<String, String> { // 返回类型更改
    let mut g = StableGraph::<String, String>::new(); // 图的边数据类型更改

    let a = g.add_node("节点A".to_string());
    let b = g.add_node("节点B".to_string());
    let c = g.add_node("节点C".to_string());
    let d = g.add_node("节点D".to_string());
    let e = g.add_node("节点E".to_string());

    g.add_edge(a, b, "边 A-B".to_string());
    g.add_edge(b, c, "边 B-C".to_string());
    g.add_edge(c, a, "边 C-A".to_string());
    g.add_edge(a, d, "边 A-D".to_string());
    g.add_edge(d, e, "边 D-E".to_string());
    g.add_edge(e, a, "边 E-A".to_string());
    g.add_edge(b, e, "边 B-E".to_string());

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