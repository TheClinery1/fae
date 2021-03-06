use crate::renderer::Renderer;
use test::Bencher;

#[bench]
fn bench_draw_100k_quads(b: &mut Bencher) {
    let mut renderer = Renderer::new(false);
    let draw_call = renderer.create_dummy_draw_call();
    b.iter(|| {
        for i in 0..100_000 {
            renderer.draw_quad(
                ((i % 1000) as f32, 0.0, 10.0, 10.0),
                (0.3, 0.3, 0.6, 0.6),
                (1.0, 1.0, 0.5, 1.0),
                (28.0, 5.0, 5.0),
                0.0,
                &draw_call,
            );
        }
        renderer.flush();
    });
}

#[bench]
fn bench_draw_quad(b: &mut Bencher) {
    let mut renderer = Renderer::new(false);
    let draw_call = renderer.create_dummy_draw_call();
    b.iter(|| {
        renderer.draw_quad(
            (0.0, 0.0, 10.0, 10.0),
            (0.3, 0.3, 0.6, 0.6),
            (1.0, 1.0, 0.5, 1.0),
            (28.0, 5.0, 5.0),
            0.0,
            &draw_call,
        );
        renderer.flush();
    });
}

#[bench]
fn bench_draw_quad_legacy(b: &mut Bencher) {
    let mut renderer = Renderer::new(true);
    let draw_call = renderer.create_dummy_draw_call();
    b.iter(|| {
        renderer.draw_quad(
            (0.0, 0.0, 10.0, 10.0),
            (0.3, 0.3, 0.6, 0.6),
            (1.0, 1.0, 0.5, 1.0),
            (28.0, 5.0, 5.0),
            0.0,
            &draw_call,
        );
        renderer.flush();
    });
}

#[bench]
fn bench_draw_quad_ninepatch(b: &mut Bencher) {
    let mut renderer = Renderer::new(false);
    let draw_call = renderer.create_dummy_draw_call();
    b.iter(|| {
        renderer.draw_quad_ninepatch(
            ((0.33, 0.33, 0.33), (0.33, 0.33, 0.33)),
            (0.0, 0.0, 10.0, 10.0),
            (0.3, 0.3, 0.6, 0.6),
            (1.0, 1.0, 0.5, 1.0),
            (28.0, 5.0, 5.0),
            0.0,
            &draw_call,
        );
        renderer.flush();
    });
}
