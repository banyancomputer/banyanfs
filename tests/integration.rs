#[test]
fn rust_test() {
    assert_eq!(1, 1);
}

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use wasm_bindgen::*;
    use wasm_bindgen_test::*;

    use wasm_bindgen_futures::JsFuture;

    #[wasm_bindgen_test(async)]
    async fn async_test() {
        // Creates a JavaScript Promise which will asynchronously resolve with the value 42.
        let promise = js_sys::Promise::resolve(&JsValue::from(42));

        // Converts that Promise into a Future. The unit test will wait for the Future to resolve.
        let result = JsFuture::from(promise).await.expect("future to resolve");
        assert_eq!(JsValue::from(42), result);
    }
}
