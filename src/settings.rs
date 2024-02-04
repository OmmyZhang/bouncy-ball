use web_sys::{Event, HtmlInputElement, InputEvent};
use yew::{function_component, html, use_state, Callback, Html, Properties, TargetCast};

#[derive(Properties, PartialEq)]
pub struct Props {
    pub v: f64,
    pub v_onchange: Callback<f64>,
    pub mw: usize,
    pub mw_onchange: Callback<usize>,
    pub mh: usize,
    pub mh_onchange: Callback<usize>,
}

#[function_component(Settings)]
pub fn settings(props: &Props) -> Html {
    let mw = use_state(|| props.mw);
    let mh = use_state(|| props.mh);
    let v = use_state(|| props.v);
    let show_setting = use_state(|| false);
    let v_oninput = {
        let v = v.clone();
        props.v_onchange.reform(move |event: InputEvent| {
            let input: HtmlInputElement = event.target_unchecked_into();
            let value = input.value_as_number();
            v.set(value);
            value
        })
    };

    let mw_onchange = {
        let mw = mw.clone();
        props.mw_onchange.reform(move |event: Event| {
            let input: HtmlInputElement = event.target_unchecked_into();
            let value = (input.value_as_number() as usize).max(4);
            mw.set(value);
            value
        })
    };

    let mh_onchange = {
        let mh = mh.clone();
        props.mh_onchange.reform(move |event: Event| {
            let input: HtmlInputElement = event.target_unchecked_into();
            let value = (input.value_as_number() as usize).max(4);
            mh.set(value);
            value
        })
    };

    let toggle_cb = {
        let show_setting = show_setting.clone();
        Callback::from(move |_| {
            show_setting.set(!*show_setting);
        })
    };

    html! {
        <div class="settings">
            <button class="toggle-btn" onclick={toggle_cb}>
                { "⚙️" }
            </button>
            if *show_setting {
                <div class="inputs">
                    <div class="size-setting">
                        <label>{ "size" }</label>
                        <input
                            type="number"
                            class="size-input"
                            value={mh.to_string()}
                            min={3}
                            onchange={mh_onchange}
                        />
                        { "×" }
                        <input
                            type="number"
                            class="size-input"
                            value={mw.to_string()}
                            min={3}
                            onchange={mw_onchange}
                        />
                    </div>
                    <div class="speed-setting">
                        <label for="speedInput">{ "speed" }</label>
                        <input
                            type="range"
                            value={v.to_string()}
                            id="speedInput"
                            min={3.0}
                            max={81.0}
                            step={0.05}
                            oninput={v_oninput}
                        />
                    </div>
                </div>
            }
        </div>
    }
}
