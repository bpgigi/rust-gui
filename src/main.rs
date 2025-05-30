use eframe::{run_native, App, CreationContext, NativeOptions};
use egui::Context;
use egui_graphs::{Graph, GraphView, DefaultNodeShape, DefaultEdgeShape, SettingsStyle}; // 引入 SettingsStyle
use petgraph::stable_graph::{StableGraph, DefaultIx};
use petgraph::Directed;

pub struct BasicApp {
    g: Graph<String, ()>,
}

impl BasicApp {
    fn new(_: &CreationContext<'_>) -> Self {
        let graph_data = generate_graph();
        Self {
            g: Graph::from(&graph_data),
        }
    }
}

impl App for BasicApp {
    fn update(&mut self, ctx: &Context, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // 配置样式以总是显示标签
            let style_settings = SettingsStyle::new().with_labels_always(true);
            let mut graph_view =
                GraphView::<String, (), Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape>::new(
                    &mut self.g,
                )
                .with_styles(&style_settings); // 应用样式
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