use game::Game;
use yew::{function_component, html, use_state, Html, Renderer};

mod game;
mod settings;

#[function_component(App)]
fn app() -> Html {
    let v_rank = use_state(|| 30_u32);

    html! {
        <Game mw={10} mh={15} />
    }
}

fn main() {
    Renderer::<App>::new().render();
}
