use eframe::Frame;
use egui::{Context, CentralPanel};
use egui_graphs::{GraphView, SettingsStyle, SettingsNavigation, SettingsInteraction, DefaultNodeShape, DefaultEdgeShape, LayoutStateRandom, LayoutRandom}; // Corrected layout imports
use crate::app::{BasicApp, AppGraph, NodePayload, EdgePayload};
use petgraph::stable_graph::DefaultIx;
use petgraph::Directed;

pub fn draw_graph_view(app: &mut BasicApp, ctx: &Context, _frame: &mut Frame) {
    CentralPanel::default().show(ctx, |ui| {
        let settings_style = SettingsStyle::new()
            .with_labels_always(app.style_labels_always);

        let settings_navigation = SettingsNavigation::new()
            .with_fit_to_screen_enabled(app.nav_fit_to_screen)
            .with_zoom_and_pan_enabled(app.nav_zoom_and_pan)
            .with_zoom_speed(app.nav_zoom_speed);

        let settings_interaction = SettingsInteraction::new()
            .with_dragging_enabled(app.ia_dragging_enabled)
            .with_node_clicking_enabled(app.ia_node_clicking_enabled)
            .with_node_selection_enabled(app.ia_node_selection_enabled)
            .with_node_selection_multi_enabled(app.ia_node_selection_multi_enabled)
            .with_edge_clicking_enabled(app.ia_edge_clicking_enabled)
            .with_edge_selection_enabled(app.ia_edge_selection_enabled)
            .with_edge_selection_multi_enabled(app.ia_edge_selection_multi_enabled);
        
        match &mut app.g {
            AppGraph::Directed(g_directed) => {
                ui.add(
                    &mut GraphView::<NodePayload, EdgePayload, Directed, DefaultIx, DefaultNodeShape, DefaultEdgeShape, LayoutStateRandom, LayoutRandom>::new(g_directed)
                        .with_interactions(&settings_interaction)
                        .with_navigations(&settings_navigation)
                        .with_styles(&settings_style)
                        .with_events(&app.event_publisher)
                );
            }
            AppGraph::Undirected(g_undirected) => {
                ui.add(
                    &mut GraphView::<NodePayload, EdgePayload, petgraph::Undirected, DefaultIx, DefaultNodeShape, DefaultEdgeShape, LayoutStateRandom, LayoutRandom>::new(g_undirected)
                        .with_interactions(&settings_interaction)
                        .with_navigations(&settings_navigation)
                        .with_styles(&settings_style)
                        .with_events(&app.event_publisher)
                );
            }
        }
    });
}