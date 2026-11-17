use std::path::Path;

use super::*;
use crate::count::Tokenizer;
use crate::functions::AstStats;

fn public(path: &str, src: &str) -> Option<usize> {
    AstStats::extract(Path::new(path), src, Tokenizer::O200k)
        .unwrap()
        .public_items
        .map(|i| i.0)
}

#[test]
fn kinds_exist() {
    for lang in Lang::ALL {
        let Some(spec) = VisibilityHelpers::spec(lang) else {
            continue;
        };
        let language = lang.language();
        for kind in spec.items.iter().chain(spec.opaque) {
            assert_ne!(language.id_for_node_kind(kind, true), 0, "{lang:?} {kind}");
        }
    }
}

#[test]
fn rust_counts_bare_pub_only() {
    let src = "pub fn a() { pub fn nested() {} }\npub(crate) fn b() {}\nfn c() {}\npub struct S;\n\
               pub(super) enum E {}\npub trait T { fn m(&self); }\nimpl S { pub fn m(&self) {} fn n(&self) {} }\n\
               pub type A = u8;\nmod inner { pub fn d() {} pub(in crate::x) fn e() {} }\n";
    assert_eq!(public("x.rs", src), Some(6));
}

#[test]
fn python_module_level_without_underscore() {
    let src = "def a():\n    def inner():\n        pass\ndef _b():\n    pass\nclass C:\n    def m(self):\n        pass\n\
               class _D:\n    pass\n@decorator\ndef e():\n    pass\nif True:\n    def f():\n        pass\n";
    assert_eq!(public("x.py", src), Some(4));
}

#[test]
fn typescript_exports() {
    let src = "export function a() {}\nfunction b() {}\nexport class C {\n  m() {}\n  private p() {}\n  #q() {}\n  protected r() {}\n}\n\
               export interface I {}\nexport type T = number;\nexport enum E { A }\nexport const f = () => 1, n = 2;\n\
               const g = 1;\nexport { b, g };\nexport { x } from './x';\nexport default function () {}\n";
    // a, C, C.m, I, T, E, f, b, g, default
    assert_eq!(public("x.ts", src), Some(10));
    let js = "export function a() {}\nexport class C { m() {} #n() {} }\nexport const v = 1;\n";
    assert_eq!(public("x.js", js), Some(3));
}

#[test]
fn go_capitalized_names() {
    let src = "package p\nfunc A() {}\nfunc b() {}\ntype S struct{}\ntype t int\nfunc (s S) M() {}\nfunc (s S) m() {}\n\
               type ( U int; v int )\ntype Alias = S\nfunc C() { type Local int }\n";
    assert_eq!(public("x.go", src), Some(6));
}

#[test]
fn keyword_languages() {
    let java = "public class C {\n  public void a() {}\n  void b() {}\n  private void c() {}\n  public C() {}\n}\n\
                interface I { void m(); private void n() {} }\n";
    assert_eq!(public("x.java", java), Some(4));
    let cs = "public class C {\n  public void A() {}\n  void B() {}\n}\ninternal class D {}\n\
              public interface I { void M(); }\n";
    assert_eq!(public("x.cs", cs), Some(4));
    let kt = "class C {\n  fun a() {}\n  private fun b() {}\n  internal fun c() {}\n}\nfun top() { fun local() {} }\n";
    assert_eq!(public("x.kt", kt), Some(3));
    let swift =
        "public class C {\n  public func a() {}\n  func b() {}\n}\nopen class D {}\nfunc e() {}\n";
    assert_eq!(public("x.swift", swift), Some(3));
    let scala = "object O {\n  def a(): Int = 1\n  private def b(): Int = 2\n}\nclass K\n";
    assert_eq!(public("x.scala", scala), Some(3));
    let php = "<?php\nfunction f() {}\nclass C {\n  public function a() {}\n  function b() {}\n  private function c() {}\n  protected function d() {}\n}\n";
    assert_eq!(public("x.php", php), Some(4));
    let zig = "pub fn a() void {}\nfn b() void {}\npub const S = struct {\n    pub fn m() void {}\n};\nconst T = struct {};\npub const n = 1;\n";
    assert_eq!(public("x.zig", zig), Some(3));
}

#[test]
fn unsupported_languages_are_none() {
    assert_eq!(public("x.c", "int f(void) { return 1; }\n"), None);
    assert_eq!(public("x.rb", "def f\nend\n"), None);
}
