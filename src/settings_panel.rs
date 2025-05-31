use egui::{Context, ScrollArea, Ui};
use crate::app::BasicApp; // Assuming BasicApp is in app.rs

pub fn draw_settings_panel(app: &mut BasicApp, ctx: &Context) {
    egui::SidePanel::right("config_panel")
        .min_width(250.0)
        .show(ctx, |ui| {
            ui.heading("配置面板");
            ui.separator();
            
            ScrollArea::vertical().show(ui, |ui_scroll| {
                draw_graph_properties_settings(app, ui_scroll);
                ui_scroll.separator();
                draw_style_settings(app, ui_scroll);
                ui_scroll.separator();
                draw_navigation_settings(app, ui_scroll);
                ui_scroll.separator();
                draw_interaction_settings(app, ui_scroll);
                ui_scroll.separator();
                draw_simulation_settings(app, ui_scroll);
                ui_scroll.separator();
                draw_node_edge_management(app, ui_scroll);
                ui_scroll.separator();
                draw_debug_info(app, ui_scroll);
            });
        });
}

fn draw_graph_properties_settings(app: &mut BasicApp, ui: &mut Ui) {
    ui.collapsing("图属性", |ui| {
        if ui.checkbox(&mut app.is_directed, "有向图").changed() {
            // Call the new conversion function instead of reset
            app.convert_graph_direction();
        }
        ui.horizontal(|ui| {
            ui.label("节点数:");
            if ui.add(egui::DragValue::new(&mut app.graph_nodes_count).speed(1.0).range(0..=1000)).changed() {
                app.reset_graph_and_simulation();
            }
        });
        ui.horizontal(|ui| {
            ui.label("边数:");
            if ui.add(egui::DragValue::new(&mut app.graph_edges_count).speed(1.0).range(0..=2000)).changed() {
                app.reset_graph_and_simulation();
            }
        });
        if ui.button("重置图").on_hover_text("使用当前节点和边数重新生成图").clicked() {
            app.reset_graph_and_simulation();
        }
    });
}

fn draw_style_settings(app: &mut BasicApp, ui: &mut Ui) {
    ui.collapsing("样式设置", |ui| {
        ui.checkbox(&mut app.style_labels_always, "总是显示标签");
    });
}

fn draw_navigation_settings(app: &mut BasicApp, ui: &mut Ui) {
    ui.collapsing("导航设置", |ui| {
        if ui.checkbox(&mut app.nav_fit_to_screen, "适应屏幕").changed() {
            if app.nav_fit_to_screen {
                app.nav_zoom_and_pan = false;
            }
        }
        ui.label("启用后，图表将始终缩放以适应屏幕。");
        ui.add_space(5.0);
        ui.add_enabled_ui(!app.nav_fit_to_screen, |ui| {
            ui.checkbox(&mut app.nav_zoom_and_pan, "缩放与平移");
            ui.label("启用后，可用Ctrl+滚轮缩放，鼠标中键拖拽平移。");
            ui.add(egui::Slider::new(&mut app.nav_zoom_speed, 0.01..=0.5).text("缩放速度"));
        });
    });
}

fn draw_interaction_settings(app: &mut BasicApp, ui: &mut Ui) {
    ui.collapsing("交互设置", |ui| {
        if ui.checkbox(&mut app.ia_dragging_enabled, "节点可拖拽").on_hover_text("启用后，可按住鼠标左键拖拽节点。").changed() {
            if app.ia_dragging_enabled { app.ia_node_clicking_enabled = true; }
        }
        ui.add_space(5.0);
        ui.add_enabled_ui(!(app.ia_dragging_enabled || app.ia_node_selection_enabled || app.ia_node_selection_multi_enabled), |ui| {
            ui.checkbox(&mut app.ia_node_clicking_enabled, "节点可点击").on_hover_text("启用后，可捕获节点点击事件。");
        }).response.on_disabled_hover_text("节点拖拽或选择已启用时，点击自动启用");
        ui.add_space(5.0);
        ui.add_enabled_ui(!app.ia_node_selection_multi_enabled, |ui| {
            if ui.checkbox(&mut app.ia_node_selection_enabled, "节点可选择").on_hover_text("启用后，可单击选择/取消选择节点。").changed() {
                 if app.ia_node_selection_enabled { app.ia_node_clicking_enabled = true; }
            }
        }).response.on_disabled_hover_text("节点多选已启用时，单选自动启用");
        ui.add_space(5.0);
        if ui.checkbox(&mut app.ia_node_selection_multi_enabled, "节点可多选").on_hover_text("启用后，可按住Ctrl点击多选节点。").changed() {
            if app.ia_node_selection_multi_enabled {
                app.ia_node_clicking_enabled = true;
                app.ia_node_selection_enabled = true;
            }
        }
        ui.add_space(10.0);
        ui.add_enabled_ui(!(app.ia_edge_selection_enabled || app.ia_edge_selection_multi_enabled), |ui| {
             ui.checkbox(&mut app.ia_edge_clicking_enabled, "边可点击").on_hover_text("启用后，可捕获边点击事件。");
        }).response.on_disabled_hover_text("边选择已启用时，点击自动启用");
        ui.add_space(5.0);
        ui.add_enabled_ui(!app.ia_edge_selection_multi_enabled, |ui| {
            if ui.checkbox(&mut app.ia_edge_selection_enabled, "边可选择").on_hover_text("启用后，可单击选择/取消选择边。").changed() {
                if app.ia_edge_selection_enabled { app.ia_edge_clicking_enabled = true; }
            }
        }).response.on_disabled_hover_text("边多选已启用时，单选自动启用");
        ui.add_space(5.0);
        if ui.checkbox(&mut app.ia_edge_selection_multi_enabled, "边可多选").on_hover_text("启用后，可按住Ctrl点击多选边。").changed() {
            if app.ia_edge_selection_multi_enabled {
                app.ia_edge_clicking_enabled = true;
                app.ia_edge_selection_enabled = true;
            }
        }
    });
}

fn draw_simulation_settings(app: &mut BasicApp, ui: &mut Ui) {
    ui.collapsing("模拟设置", |ui| {
        ui.checkbox(&mut app.simulation_stopped, "停止模拟");
        ui.add_enabled_ui(!app.simulation_stopped, |ui| {
            if ui.add(egui::Slider::new(&mut app.sim_dt, 0.001..=0.1).text("时间步长 (dt)")).changed() ||
               ui.add(egui::Slider::new(&mut app.sim_cooloff_factor, 0.5..=0.999).text("冷却因子")).changed() ||
               ui.add(egui::Slider::new(&mut app.sim_scale, 10.0..=500.0).text("缩放尺度")).changed() {
                // Update simulation parameters if they are changed
                app.force_algo.conf.dt = app.sim_dt;
                app.force_algo.conf.cooloff_factor = app.sim_cooloff_factor;
                app.force_algo.conf.scale = app.sim_scale;
            }
        });
    });
}

fn draw_node_edge_management(app: &mut BasicApp, ui: &mut Ui) {
    // Temporary state for weight input, ideally part of app state or passed differently
    // For simplicity in this step, we'll use local mutable state if possible,
    // or add temporary fields to BasicApp if needed for TextEdit.
    // Let's add input_node_weight and input_edge_weight to BasicApp for now.
    // These should be initialized in BasicApp::new()

    ui.collapsing("节点/边管理", |ui| {
        ui.label("添加节点:");
        ui.horizontal(|ui| {
            ui.label("标签:");
            ui.text_edit_singleline(&mut app.input_node_to_add);
        });
        ui.horizontal(|ui| {
            ui.label("权重:");
            // Assuming app has input_node_weight: f32
            ui.add(egui::DragValue::new(&mut app.input_node_weight).speed(0.1).range(0.0..=100.0));
        });
        if ui.button("添加节点").clicked() {
            app.add_node_ui(app.input_node_to_add.clone(), app.input_node_weight);
            app.input_node_to_add.clear();
            // app.input_node_weight = 1.0; // Reset to default
        }
        ui.add_space(5.0);

        ui.label("删除节点:");
        ui.horizontal(|ui| {
            ui.label("标签:");
            ui.text_edit_singleline(&mut app.input_node_to_remove);
            if ui.button("删除").clicked() {
                app.remove_node_ui(app.input_node_to_remove.clone());
                app.input_node_to_remove.clear();
            }
        });
        
        ui.separator();
        ui.label("添加边 (通过标签):");
        ui.horizontal(|ui| {
            ui.label("从:");
            ui.text_edit_singleline(&mut app.input_node_from);
            ui.label("到:");
            ui.text_edit_singleline(&mut app.input_node_to);
        });
        ui.horizontal(|ui| {
            ui.label("权重:");
            // Assuming app has input_edge_weight: f32
            ui.add(egui::DragValue::new(&mut app.input_edge_weight).speed(0.1).range(0.0..=100.0));
        });
        if ui.button("添加边").clicked() {
            app.add_edge_ui(app.input_node_from.clone(), app.input_node_to.clone(), app.input_edge_weight);
            app.input_node_from.clear();
            app.input_node_to.clear();
            // app.input_edge_weight = 1.0; // Reset to default
        }
        ui.add_space(5.0);
        
        ui.separator();
        if ui.button("在选中节点间添加边").on_hover_text("选择两个节点后点击此按钮添加边").clicked() {
            app.add_edge_between_selected_nodes();
        }
        if ui.button("删除选中的边").on_hover_text("选择一条或多条边后点击此按钮删除").clicked() {
            app.remove_selected_edges_ui();
        }
        ui.separator();
        draw_selected_element_properties(app, ui); // New function to draw selected element props
    });
}

// New function to display/edit properties of selected node/edge
fn draw_selected_element_properties(app: &mut BasicApp, ui: &mut Ui) {
    ui.label("选中元素属性:");

    let selected_nodes: Vec<_> = match &app.g {
        crate::app::AppGraph::Directed(g) => g.selected_nodes().iter().copied().collect(),
        crate::app::AppGraph::Undirected(g) => g.selected_nodes().iter().copied().collect(),
    };
    let selected_edges: Vec<_> = match &app.g {
        crate::app::AppGraph::Directed(g) => g.selected_edges().iter().copied().collect(),
        crate::app::AppGraph::Undirected(g) => g.selected_edges().iter().copied().collect(),
    };

    if selected_nodes.len() == 1 && selected_edges.is_empty() {
        let node_idx = selected_nodes[0];
        if let Some(node_payload) = app.get_node_payload_mut(node_idx) {
            ui.label(format!("节点: {}", node_payload.label));
            if ui.add(egui::DragValue::new(&mut node_payload.weight).speed(0.1).prefix("权重: ")).changed() {
                // Clone the payload *after* DragValue has modified it, then pass the clone.
                // This releases the mutable borrow of node_payload before calling another &mut self method.
                let updated_payload = node_payload.clone();
                app.update_fdg_node_payload(node_idx, updated_payload);
            }
        }
    } else if selected_edges.len() == 1 && selected_nodes.is_empty() {
        let edge_idx = selected_edges[0];
         if let Some(edge_payload) = app.get_edge_payload_mut(edge_idx) {
            ui.label(format!("边: {}", edge_payload.label));
            if ui.add(egui::DragValue::new(&mut edge_payload.weight).speed(0.1).prefix("权重: ")).changed() {
                // Clone the payload *after* DragValue has modified it.
                let updated_payload = edge_payload.clone();
                app.update_fdg_edge_payload(edge_idx, updated_payload);
            }
        }
    } else if selected_nodes.len() > 1 || selected_edges.len() > 1 || (!selected_nodes.is_empty() && !selected_edges.is_empty()) {
        ui.label("请只选择单个节点或单个边以编辑属性。");
    } else {
        ui.label("未选择任何元素或选择不明确。");
    }
}


fn draw_debug_info(app: &BasicApp, ui: &mut Ui) {
    ui.collapsing("调试信息", |ui| {
        let (num_nodes, num_edges) = match &app.g {
            crate::app::AppGraph::Directed(g) => (g.node_count(), g.edge_count()),
            crate::app::AppGraph::Undirected(g) => (g.node_count(), g.edge_count()),
        };
        ui.label(format!("Egui图: {} 节点, {} 边", num_nodes, num_edges));
        ui.label(format!("Fdg图: {} 节点, {} 边", app.sim.node_count(), app.sim.edge_count()));
        ui.label(format!("标签映射数量: {}", app.node_label_to_index_map.len()));
        
        ui.separator();
        ui.label("选中的节点:");
        let selected_nodes_labels: Vec<String> = match &app.g {
            crate::app::AppGraph::Directed(g) => g.selected_nodes().iter().filter_map(|idx| g.node(*idx).map(|n| n.payload().label.clone())).collect(),
            crate::app::AppGraph::Undirected(g) => g.selected_nodes().iter().filter_map(|idx| g.node(*idx).map(|n| n.payload().label.clone())).collect(),
        };
        if selected_nodes_labels.is_empty() {
            ui.label("无");
        } else {
            for label in selected_nodes_labels {
                ui.label(format!("- {}", label));
            }
        }

        ui.separator();
        ui.label("选中的边:");
        let selected_edges_details: Vec<String> = match &app.g {
            crate::app::AppGraph::Directed(g) => g.selected_edges().iter().filter_map(|idx| g.edge(*idx).map(|e| e.payload().label.clone())).collect(),
            crate::app::AppGraph::Undirected(g) => g.selected_edges().iter().filter_map(|idx| g.edge(*idx).map(|e| e.payload().label.clone())).collect(),
        };
        if selected_edges_details.is_empty() {
            ui.label("无");
        } else {
            for detail in selected_edges_details {
                ui.label(format!("- {}", detail));
            }
        }
    });
}