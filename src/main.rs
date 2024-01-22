use game::Game;
use yew::{function_component, html, Html, Renderer};

mod game;
mod settings;

#[function_component(App)]
fn app() -> Html {
    html! {
        <Game mw={10} mh={15} />
    }
}

fn main() {
    Renderer::<App>::new().render();
}
