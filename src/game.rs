use std::collections::VecDeque;

use gloo_console::log;
use gloo_timers::callback::Interval;
use web_sys::wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, Event, HtmlCanvasElement, HtmlImageElement};
use yew::{
    function_component, html, use_effect_with, use_mut_ref, use_node_ref, use_state, Callback,
    Html, Properties, TargetCast,
};

const BALL_SIZE: f64 = 36.0;
const BLOCK_SIZE: f64 = 100.0;
const BLOCK_BORDER: f64 = 5.0;

const BG_COLOR: &str = "#5050e0";

const INTERV: u32 = 8;
const NEXT_BALL_TIME: u32 = 10;
const V: f64 = 5.0;

const NEW_BALL_ID: u32 = 99999;

macro_rules! clone_all {
    [$($s:ident), *] => {
        $(
            let $s = $s.clone();
        )*
    };
}

#[derive(Default, Debug)]
struct BallStatus {
    x: f64,
    y: f64,
    to_up: bool,
    // 只是表示与初始方向是否一致
    // 向右的速度也可能是负的
    to_right: bool,
}

#[derive(Default)]
struct MapStatus {
    ctx: Option<CanvasRenderingContext2d>,
    img: Option<HtmlImageElement>,
    moving_balls: Vec<BallStatus>,
    block_map: VecDeque<Vec<u32>>,
    n_waiting_bolls: u32,
    mw: usize,
    mh: usize,
    waiting_next: u32,
    vx: f64,
    vy: f64,
}

impl MapStatus {
    fn update_blocks_and_check_game_end(&mut self, level: u32) -> bool {
        let last_line = self.block_map.pop_back().unwrap();
        log!(JsValue::from_str(&format!("{:#?}", &last_line)));
        if last_line.into_iter().any(|v| v > 0) {
            return true;
        }

        let new_line = vec![level; self.mw]; // TODO
        log!(JsValue::from_str(&format!("{:#?}", &new_line)));
        self.block_map.push_front(new_line);
        log!(JsValue::from_str(&format!("{}", self.block_map.len())));

        self.draw_basic();

        false
    }

    fn draw_ball(&self, ox: f64, oy: f64) {
        if let (Some(ctx), Some(img)) = (self.ctx.as_ref(), self.img.as_ref()) {
            ctx.draw_image_with_html_image_element_and_dw_and_dh(
                img,
                ox - BALL_SIZE / 2.0,
                oy - BALL_SIZE / 2.0,
                BALL_SIZE,
                BALL_SIZE,
            )
            .expect("draw next ball failed");
        }
    }

    fn block_color(&self, v: u32) -> String {
        format!(
            "rgb({}, 20, {})",
            (v * 5 + 20).min(250),
            (250 * 7 - v).max(20)
        )
    }

    fn draw_block(&self, i: usize, j: usize, v: u32) {
        // TODO: use offscreen canvas
        let ctx = self.ctx.as_ref().unwrap();
        if v > 0 {
            let x = j as f64 * BLOCK_SIZE;
            let y = i as f64 * BLOCK_SIZE;

            ctx.set_fill_style(&JsValue::from_str("#e0e0e0"));
            ctx.begin_path();
            ctx.move_to(x, y);
            ctx.line_to(x + BLOCK_SIZE, y);
            ctx.line_to(x, y + BLOCK_SIZE);
            ctx.fill();

            ctx.set_fill_style(&JsValue::from_str("#202020"));
            ctx.begin_path();
            ctx.move_to(x + BLOCK_SIZE, y);
            ctx.line_to(x, y + BLOCK_SIZE);
            ctx.line_to(x + BLOCK_SIZE, y + BLOCK_SIZE);
            ctx.fill();

            ctx.set_fill_style(&JsValue::from_str(&self.block_color(v)));
            ctx.fill_rect(
                x + BLOCK_BORDER,
                y + BLOCK_BORDER,
                BLOCK_SIZE - BLOCK_BORDER - BLOCK_BORDER,
                BLOCK_SIZE - BLOCK_BORDER - BLOCK_BORDER,
            );
            let text = v.to_string();
            ctx.set_fill_style(&JsValue::from_str("white"));
            ctx.fill_text(
                &text,
                x + (BLOCK_SIZE - ctx.measure_text(&text).unwrap().width()) / 2.0,
                y + BLOCK_SIZE / 2.0,
            )
            .unwrap();
        }
    }

    fn draw_basic(&self) {
        let ww = self.mw as f64 * BLOCK_SIZE;
        let hh = self.mh as f64 * BLOCK_SIZE;
        let ctx = self.ctx.as_ref().unwrap();
        ctx.set_fill_style(&JsValue::from_str(BG_COLOR));
        ctx.fill_rect(0.0, 0.0, ww, hh);
        for i in 0..self.mh {
            for j in 0..self.mw {
                self.draw_block(i, j, self.block_map[i][j])
            }
        }
    }

    pub fn simulate_moving(&mut self) -> Option<(u32, bool)> {
        let ww = self.mw as f64 * BLOCK_SIZE;
        let hh = self.mh as f64 * BLOCK_SIZE;

        self.draw_basic();

        let mut new_ball = 0;

        let mut need_remove = vec![];

        for (idx, ball) in self.moving_balls.iter_mut().enumerate() {
            let pi = (ball.y / BLOCK_SIZE).floor() as usize;
            let pj = (ball.x / BLOCK_SIZE).floor() as usize;

            let have_left = pj > 0 && self.block_map[pi][pj - 1] > 0;
            let have_right = pj < self.mw - 1 && self.block_map[pi][pj + 1] > 0;
            let have_above = pi > 0 && self.block_map[pi - 1][pj] > 0;
            let have_below = pi < self.mh - 1 && self.block_map[pi + 1][pj] > 0;

            let min_x = if have_left {
                pj as f64 * BLOCK_SIZE
            } else {
                0.0
            } + BALL_SIZE / 2.0;
            let max_x = if have_right {
                (pj + 1) as f64 * BLOCK_SIZE
            } else {
                ww
            } - BALL_SIZE / 2.0;
            let min_y = if have_above {
                pi as f64 * BLOCK_SIZE
            } else {
                0.0
            } + BALL_SIZE / 2.0;
            let max_y = if have_below {
                (pi + 1) as f64 * BLOCK_SIZE
            } else {
                hh
            } - BALL_SIZE / 2.0;

            let new_x = ball.x + if ball.to_right { self.vx } else { -self.vx };
            let new_y = ball.y + if ball.to_up { self.vy } else { -self.vy };

            let real_new_x = if new_x < min_x {
                if have_left {
                    self.block_map[pi][pj - 1] -= 1;
                }
                2.0 * min_x - new_x
            } else if new_x > max_x {
                if have_right {
                    self.block_map[pi][pj + 1] -= 1;
                }
                2.0 * max_x - new_x
            } else {
                new_x
            };
            let real_new_y = if new_y < min_y {
                if have_above {
                    self.block_map[pi - 1][pj] -= 1;
                }
                2.0 * min_y - new_y
            } else if new_y > max_y {
                if have_below {
                    self.block_map[pi + 1][pj] -= 1;
                }
                2.0 * max_y - new_y
            } else {
                new_y
            };

            //log!(JsValue::from_str(&format!("{} {}", new_y, max_y)));
            //log!(JsValue::from_str(&format!("{} <? {}", new_y, max_y)));

            ball.x = real_new_x;
            ball.y = real_new_y;

            if new_y < min_y || new_y > max_y {
                ball.to_up = !ball.to_up;
            }
            if new_x < min_x || new_x > max_x {
                ball.to_right = !ball.to_right;
            }

            if new_y > hh - BALL_SIZE / 2.0 {
                need_remove.push(idx);
            }
        }

        for idx in need_remove.iter() {
            self.moving_balls.remove(*idx);
        }

        // log!(JsValue::from_str(&format!("{:#?}", self.moving_balls)));
        // log!(JsValue::from_str(&format!("{:#?}", self.n_waiting_bolls)));

        if self.n_waiting_bolls > 0 {
            self.draw_ball(ww / 2.0, hh - BALL_SIZE / 2.0);
            if self.waiting_next == 0 {
                self.moving_balls.push(BallStatus {
                    x: ww / 2.0,
                    y: hh - BALL_SIZE / 2.0,
                    to_up: true,
                    to_right: true,
                });
                self.n_waiting_bolls -= 1;
                self.waiting_next = NEXT_BALL_TIME;
            } else {
                self.waiting_next -= 1;
            }
        }

        for ball in self.moving_balls.iter() {
            self.draw_ball(ball.x, ball.y);
        }

        Some((0, self.n_waiting_bolls == 0 && self.moving_balls.is_empty()))
    }
}

#[derive(Properties, PartialEq)]
pub struct Props {
    /// map width, number of blocks
    pub mw: usize,
    /// map height, numher for blocks
    pub mh: usize,
}

#[function_component(Game)]
pub fn game(props: &Props) -> Html {
    let n_balls = use_state(|| 0_u32);
    let level = use_state(|| 1_u32);

    let canvas_ref = use_node_ref();

    let is_moving = use_state(|| false);
    let map_status = use_mut_ref(MapStatus::default);
    let simulation_interval = use_mut_ref(|| None);

    // 点击
    let onclick = {
        clone_all![is_moving, map_status, simulation_interval, n_balls, level];
        Callback::from(move |event| {
            if *is_moving {
                return;
            }

            is_moving.set(true);

            map_status.borrow_mut().vx = 0.8 * V;
            map_status.borrow_mut().vy = -0.6 * V;
            map_status.borrow_mut().n_waiting_bolls = *n_balls;

            *simulation_interval.borrow_mut() = {
                clone_all![map_status, simulation_interval, n_balls, is_moving, level];
                Some(Interval::new(INTERV, move || {
                    // 保险起见，万一上一个没跑完
                    if let Some(ms) = map_status.try_borrow_mut().ok().as_deref_mut() {
                        let (n_new_balls, done) = ms.simulate_moving().unwrap_or_default();
                        if n_new_balls > 0 {
                            n_balls.set(*n_balls + n_new_balls);
                        }
                        if done {
                            is_moving.set(false);
                            *simulation_interval.borrow_mut() = None;
                            level.set(*level + 1);
                        }
                    }
                }))
            };
        })
    };

    // 载入图片
    let img_onload = {
        clone_all![map_status];
        Callback::from(move |event: Event| {
            let ball = event.target_dyn_into::<HtmlImageElement>().unwrap();
            map_status.borrow_mut().img = Some(ball);
        })
    };

    {
        clone_all![canvas_ref, map_status, n_balls, level];
        use_effect_with(
            (canvas_ref, props.mw, props.mh),
            move |(canvas_ref, mw, mh)| {
                let (mw, mh) = (*mw, *mh);
                let canvas = canvas_ref
                    .cast::<HtmlCanvasElement>()
                    .expect("canvas_ref not attached");

                let w = mw as u32 * BLOCK_SIZE as u32;
                let h = mh as u32 * BLOCK_SIZE as u32;
                canvas.set_width(w);
                canvas.set_height(h);

                let ctx = CanvasRenderingContext2d::from(JsValue::from(
                    canvas.get_context("2d").unwrap(),
                ));

                ctx.set_fill_style(&JsValue::from_str(BG_COLOR));
                ctx.set_font("45px  sans-serif");
                ctx.set_text_baseline("middle");
                ctx.fill_rect(0.0, 0.0, w as f64, h as f64);

                map_status.borrow_mut().ctx = Some(ctx);
                map_status.borrow_mut().moving_balls = vec![];
                map_status.borrow_mut().block_map = vec![vec![0; mw]; mh].into();
                map_status.borrow_mut().mw = mw;
                map_status.borrow_mut().mh = mh;
                map_status.borrow_mut().waiting_next = 0;

                n_balls.set(5);
                level.set(1);
            },
        );
    }

    // level上涨时重新生成新的一排
    {
        clone_all![level, map_status];
        use_effect_with(*level, move |level| {
            if map_status
                .borrow_mut()
                .update_blocks_and_check_game_end(*level)
            {}
        });
    }

    html! {
        <div class="container">
            <div class="no-select">
                <img id="ballImage" src="static/ball.png" onload={img_onload} />
                <span id="score">{ *n_balls }</span>
                <span id="level">{ * level }</span>
            </div>
            <canvas
                ref={canvas_ref}
                onclick={onclick}
            />
        </div>
    }
}
