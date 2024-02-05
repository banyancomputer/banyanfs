#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn web_test() {
        assert_eq!(1, 1);
    }
}
