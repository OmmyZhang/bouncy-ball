use web_sys::{HtmlInputElement, InputEvent};
use yew::{function_component, html, use_state, Callback, Html, Properties, TargetCast};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub v: f64,
    pub v_onchange: Callback<f64>,
}

#[function_component(Settings)]
pub fn settings(props: &Props) -> Html {
    let v = use_state(|| props.v);
    let oninput = {
        let v = v.clone();
        props.v_onchange.reform(move |event: InputEvent| {
            let input: HtmlInputElement = event.target_unchecked_into();
            let value = input.value_as_number();
            v.set(value);
            value
        })
    };

    html! {
        <div class="settings">
            <input type="range"
                value={v.to_string()}
                class="v__input"
                min={3.0}
                max={81.0}
                step={0.05}
                {oninput}
            />
        </div>
    }
}
