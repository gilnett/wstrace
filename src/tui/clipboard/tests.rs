use super::*;

#[test]
fn test_clipboard_invocation_safeness() {
    // Verification test: calling clipboard function does not crash or corrupt memory
    let _ = set_clipboard_text("wstrace-test-payload");
}
