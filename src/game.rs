use std::collections::VecDeque;

use gloo_console::log;
use gloo_timers::callback::Interval;
use rand::seq::SliceRandom;
use rand::Rng;
use web_sys::wasm_bindgen::JsValue;
use web_sys::{CanvasRenderingContext2d, Event, HtmlCanvasElement, HtmlImageElement, PointerEvent};
use yew::{
    function_component, html, use_effect_with, use_mut_ref, use_node_ref, use_state, Callback,
    Html, Properties, TargetCast,
};

use crate::settings::Settings;

const BALL_SIZE: f64 = 36.0;
const BALL_R: f64 = BALL_SIZE / 2.0;
const BLOCK_SIZE: f64 = 100.0;
const BLOCK_BORDER: f64 = 6.0;

const BG_COLOR: &str = "#5050e0";

const INTERV: u32 = 8;
const NEXT_BALL_TIME_DIST: f64 = 3.0 * BALL_SIZE;

const NEW_BALL_ID: u32 = 99999;

const EPS: f64 = 1e-10;

macro_rules! clone_all {
    [$($s:ident), *] => {
        $(
            let $s = $s.clone();
        )*
    };
}

#[derive(Debug)]
enum BallMovingStatus {
    Runing,
    Backing,
    Done,
}

#[derive(Debug)]
struct BallStatus {
    x: f64,
    y: f64,
    to_up: bool,
    // 只是表示与初始方向是否一致
    // 向右的速度也可能是负的
    to_right: bool,
    moving_status: BallMovingStatus,
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
    start_x: f64,
    new_start_x: Option<f64>,
}

impl MapStatus {
    fn update_blocks_and_check_game_end(&mut self, level: u32) -> bool {
        let last_line = self.block_map.pop_back().unwrap();
        if last_line.into_iter().any(|v| v > 0) {
            return true;
        }

        let n_rank = level as usize;
        let mut rng = rand::thread_rng();
        let n = rng.gen_range(
            (n_rank / 10 + 1).min(self.mw / 2)..(n_rank / 5 + n_rank.min(3) + 2).min(self.mw),
        );
        let mut new_line: Vec<u32> = (0..self.mw)
            .map(|idx| if idx < n { level } else { 0 })
            .collect();
        new_line.shuffle(&mut rng);
        self.block_map.push_front(new_line);

        self.draw_basic(true);

        false
    }

    fn draw_ball(&self, ox: f64, oy: f64) {
        if let (Some(ctx), Some(img)) = (self.ctx.as_ref(), self.img.as_ref()) {
            ctx.draw_image_with_html_image_element_and_dw_and_dh(
                img,
                ox - BALL_R,
                oy - BALL_R,
                BALL_SIZE,
                BALL_SIZE,
            )
            .expect("draw next ball failed");
        }
    }

    fn block_color(&self, v: u32) -> String {
        format!(
            "rgb({}, {}, {})",
            20 + v % 20 * 13,
            (80.0 + ((v as f64 / 31.0).sin()) * 80.0) as u8,
            (110.0 + ((v as f64 / 47.0).cos()) * 80.0) as u8,
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

    fn draw_basic(&self, with_start_ball: bool) {
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
        if with_start_ball {
            self.draw_ball(self.start_x, hh - BALL_R)
        };
    }

    pub fn simulate_moving(&mut self, v: f64) -> (u32, bool) {
        let hh = self.mh as f64 * BLOCK_SIZE;

        let mut new_ball = 0;

        for ball in self.moving_balls.iter_mut() {
            match ball.moving_status {
                BallMovingStatus::Done => {}
                BallMovingStatus::Backing => {
                    let new_start_x = self.new_start_x.unwrap();
                    let rest_lx = new_start_x - ball.x;
                    if rest_lx.abs() < EPS {
                        ball.moving_status = BallMovingStatus::Done;
                    } else {
                        ball.x += rest_lx.abs().min(v.abs() * 0.4).copysign(rest_lx);
                    }
                    log!(JsValue::from_str(&format!("~~ backing: {}", ball.x)));
                }
                BallMovingStatus::Runing => {
                    let mut rest_lx = v * if ball.to_right { self.vx } else { -self.vx };
                    let mut rest_ly = v * if ball.to_up { self.vy } else { -self.vy };

                    // 为了方便检测碰撞，每次只走不离开当前格子的距离
                    while rest_lx.abs() > EPS && rest_ly.abs() > EPS {
                        let pi = (ball.y + (BALL_R + EPS).copysign(rest_ly)).div_euclid(BLOCK_SIZE)
                            as usize;
                        let rpi = ball.y.div_euclid(BLOCK_SIZE) as usize;
                        let pj = (ball.x + (BALL_R + EPS).copysign(rest_lx)).div_euclid(BLOCK_SIZE)
                            as usize;
                        let rpj = ball.x.div_euclid(BLOCK_SIZE) as usize;

                        if self.block_map[rpi][rpj] > 0 {
                            panic!("!!!! {}, {}", pi, pj);
                        }

                        let max_lx = BLOCK_SIZE.mul_add(
                            if rest_lx.is_sign_positive() {
                                pj + 1
                            } else {
                                pj
                            } as f64,
                            -ball.x - BALL_R.copysign(rest_lx),
                        );

                        let max_ly = BLOCK_SIZE.mul_add(
                            if rest_ly.is_sign_positive() {
                                pi + 1
                            } else {
                                pi
                            } as f64,
                            -ball.y - BALL_R.copysign(rest_ly),
                        );

                        let (lx, ly, reach_x, reach_y) =
                            if rest_lx.abs() < max_lx.abs() && rest_ly.abs() < max_ly.abs() {
                                (rest_lx, rest_ly, false, false)
                            } else if (max_lx * rest_ly).abs() < (max_ly * rest_lx).abs() {
                                (max_lx, max_lx / rest_lx * rest_ly, true, false)
                            } else {
                                (max_ly / rest_ly * rest_lx, max_ly, false, true)
                            };

                        let (next_pj, exist_next_pj) = if lx.is_sign_positive() {
                            (pj + 1, pj < self.mw - 1)
                        } else {
                            (pj - 1, pj > 0)
                        };

                        let (next_pi, exist_next_pi) = if ly.is_sign_positive() {
                            (pi + 1, pi < self.mh - 1)
                        } else {
                            (pi - 1, pi > 0)
                        };

                        /*
                        log!(JsValue::from_str(&format!(
                            "{} {}\n{}, {} | {}, {} ({} {}) => {}, {}\n {}, {} -> {}, {}",
                            ball.x,
                            ball.y,
                            rest_lx,
                            rest_ly,
                            max_lx,
                            max_ly,
                            reach_x,
                            reach_y,
                            lx,
                            ly,
                            pi,
                            pj,
                            next_pi,
                            next_pj
                        )));
                        */

                        rest_lx -= lx;
                        rest_ly -= ly;

                        ball.x += lx;
                        ball.y += ly;

                        if reach_x {
                            if !exist_next_pj {
                                ball.to_right = !ball.to_right;
                                rest_lx = -rest_lx;
                            } else if self.block_map[rpi][next_pj] > 0 {
                                ball.to_right = !ball.to_right;
                                rest_lx = -rest_lx;
                                self.block_map[rpi][next_pj] -= 1;
                            } else if self.block_map[pi][next_pj] > 0 {
                                // 撞角近似为撞边
                                ball.to_right = !ball.to_right;
                                rest_lx = -rest_lx;
                                self.block_map[pi][next_pj] -= 1;
                            }
                        }

                        if reach_y {
                            if !exist_next_pi {
                                ball.to_up = !ball.to_up;
                                rest_ly = -rest_ly;
                            } else if self.block_map[next_pi][rpj] > 0 {
                                ball.to_up = !ball.to_up;
                                rest_ly = -rest_ly;
                                self.block_map[next_pi][rpj] -= 1;
                            } else if self.block_map[next_pi][pj] > 0 {
                                ball.to_up = !ball.to_up;
                                rest_ly = -rest_ly;
                                self.block_map[next_pi][pj] -= 1;
                            }

                            if next_pi == self.mh {
                                rest_lx = 0.0;
                                rest_ly = 0.0;
                                if self.new_start_x.is_some() {
                                    ball.moving_status = BallMovingStatus::Backing;
                                    log!(JsValue::from_str(&format!(
                                        "backing: {} {}",
                                        ball.x, ball.y
                                    )));
                                } else {
                                    self.new_start_x = Some(ball.x);
                                    ball.moving_status = BallMovingStatus::Done;
                                    log!(JsValue::from_str(&format!(
                                        "done: {} {}",
                                        ball.x, ball.y
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }

        // log!(JsValue::from_str(&format!("{:#?}", self.moving_balls)));
        // log!(JsValue::from_str(&format!("{:#?}", self.n_waiting_bolls)));

        if self.n_waiting_bolls > 0 {
            if self.waiting_next == 0 {
                let go_more = self.moving_balls.len() as f64
                    / (self.n_waiting_bolls as f64 + self.moving_balls.len() as f64)
                    / 10.0;
                self.moving_balls.push(BallStatus {
                    x: self.start_x + self.vx * NEXT_BALL_TIME_DIST * go_more,
                    y: hh - BALL_R + self.vy * NEXT_BALL_TIME_DIST * go_more,
                    to_up: true,
                    to_right: true,
                    moving_status: BallMovingStatus::Runing,
                });
                self.n_waiting_bolls -= 1;
                self.waiting_next = (NEXT_BALL_TIME_DIST / v) as u32;
            } else {
                self.waiting_next -= 1;
            }
        }

        self.draw_basic(self.n_waiting_bolls > 0);
        self.moving_balls.iter().for_each(|ball| {
            self.draw_ball(ball.x, ball.y);
        });

        let done = self.n_waiting_bolls == 0
            && self
                .moving_balls
                .iter()
                .all(|ball| matches!(ball.moving_status, BallMovingStatus::Done));
        if done {
            self.start_x = self.new_start_x.unwrap();
            self.new_start_x = None;
        }

        (new_ball, done)
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

    let v = use_mut_ref(|| 7.0);

    let v_onchange = {
        let v = v.clone();
        Callback::from(move |new_v| {
            *v.borrow_mut() = new_v;
        })
    };

    // 点击
    let onclick = {
        clone_all![
            is_moving,
            map_status,
            simulation_interval,
            n_balls,
            level,
            v,
            canvas_ref
        ];
        Callback::from(move |event: PointerEvent| {
            if *is_moving {
                return;
            }
            let (x, y) = (event.client_x() as f64, event.client_y() as f64);

            let canvas = canvas_ref
                .cast::<HtmlCanvasElement>()
                .expect("canvas_ref not attached");
            let rect = canvas.get_bounding_client_rect();
            let ratio = rect.width() / (map_status.borrow().mw as f64 * BLOCK_SIZE);

            let (ox, oy) = (
                rect.left() + map_status.borrow().start_x * ratio,
                rect.bottom() - BALL_R * ratio,
            );

            let (dx, dy) = (x - ox, y - oy);
            if dy > BALL_R * ratio {
                return;
            }

            map_status.borrow_mut().vx = dx / dx.hypot(dy);
            map_status.borrow_mut().vy = dy / dx.hypot(dy);

            is_moving.set(true);
            map_status.borrow_mut().moving_balls = vec![];
            map_status.borrow_mut().n_waiting_bolls = *n_balls;

            *simulation_interval.borrow_mut() = {
                clone_all![
                    map_status,
                    simulation_interval,
                    n_balls,
                    is_moving,
                    level,
                    v
                ];
                Some(Interval::new(INTERV, move || {
                    let v = *v.borrow();
                    // 保险起见，万一上一个没跑完
                    if let Some(ms) = map_status.try_borrow_mut().ok().as_deref_mut() {
                        let (n_new_balls, done) = ms.simulate_moving(v);
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
            let ball: HtmlImageElement = event.target_unchecked_into();
            map_status.borrow_mut().img = Some(ball);
            map_status.borrow_mut().update_blocks_and_check_game_end(1);
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

                let mut ms = map_status.borrow_mut();
                ms.ctx = Some(ctx);
                ms.moving_balls = vec![];
                ms.block_map = vec![vec![0; mw]; mh].into();
                ms.mw = mw;
                ms.mh = mh;
                ms.waiting_next = 0;
                ms.start_x = mw as f64 * BLOCK_SIZE / 2.0;

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
        <div class="game-container">
            <div class="no-select">
                <img id="ballImage" src="static/ball.png" onload={img_onload} />
                <span id="score">{ *n_balls }</span>
                <span id="level">{ * level }</span>
            </div>
            <canvas
                ref={canvas_ref}
                onpointerdown={onclick}
            />
            <Settings v={*v.borrow()} {v_onchange} />
        </div>
    }
}
