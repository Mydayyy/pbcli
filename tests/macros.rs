// https://youtrack.jetbrains.com/issue/RUST-3672/quickcheck-macro-is-not-detected-as-test-function
#[macro_export]
macro_rules! pbtest {
    ($name:ident, $fun:expr) => {
        #[test]
        fn $name() {
            const INSTANCE: &str = "https://paste.ononoki.org";

            let mut pbcli = Command::cargo_bin("pbcli").unwrap();

            let mut configured_pbcli = pbcli.arg("--host").arg(INSTANCE);

            $fun(configured_pbcli);
        }
    };
}
