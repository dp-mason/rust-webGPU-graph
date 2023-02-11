mod rendering;
mod citegraph;

use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    // initialize the citation graph
    let cite_graph = citegraph::CitationGraph::new();
    

    // setup rendering state and attach a window that can be rendered to
    let (mut event_loop, mut render_state) = rendering::State::init_rendering().await;
    
    event_loop.run( move |event, _, control_flow| {
        // TODO: write stuff for handling event for citation graph
        
        render_state.handle_rendering_events(event, control_flow);
    });
}