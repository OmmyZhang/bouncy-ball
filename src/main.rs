use game::Game;
use web_sys::window;
use yew::{function_component, html, use_memo, Html, Renderer};

mod game;
mod settings;

#[function_component(App)]
fn app() -> Html {
    let mw = 10;
    let mh = use_memo((), |_| {
        let wsize = window()
            .unwrap()
            .inner_width()
            .unwrap()
            .as_f64()
            .unwrap()
            .min(500.0);
        let hsize = window().unwrap().inner_height().unwrap().as_f64().unwrap() - 60.0;

        (mw as f64 * hsize / wsize).floor() as usize
    });
    html! {
        <Game {mw} mh={*mh} />
    }
}

fn main() {
    Renderer::<App>::new().render();
}
