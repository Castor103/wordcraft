//! Simple editing: a host that frames WordCraft as a plain body editor (beside its own form, say)
//! shows the page and one row of buttons, and no word-processor chrome.
//!
//! A host turns it on with [`WordApp::simple_edit`]; no command does, so nothing a person types
//! can turn it off. The window then draws no title bar, ribbon, File view, panes, rulers or status bar;
//! a row above the page offers Undo, Redo, Bold, Italic, Underline, Bullets and Numbering, each
//! greyed out where the document's editing restrictions refuse it. The keyboard keeps typing,
//! moving, selecting, deleting, the clipboard and those commands' own shortcuts; every other
//! shortcut (formatting, inserting, dialogs, panes, files) does nothing. The context menu offers
//! Cut, Copy and Paste, and every paste keeps the text only, so a copy from another document
//! cannot bring its fonts and colours in. A host page's own commands (`window.wordcraft`) are
//! not limited: the host sets restrictions and fills controls through them.

use egui::{Ui, vec2};
use serde_json::json;

use crate::WordApp;
use crate::theme::Tokens;

/// The buttons above the page: (icon, tooltip, command).
pub const BUTTONS: [(&str, &str, &str); 7] = [
    ("undo", "Undo", "edit.undo"),
    ("redo", "Redo", "edit.redo"),
    ("bold", "Bold", "format.bold"),
    ("italic", "Italic", "format.italic"),
    ("underline", "Underline", "format.underline"),
    ("bullets", "Bullets", "para.bullets"),
    ("numbering", "Numbering", "para.numbering"),
];

/// Commands a key may run in simple editing, besides moving the caret and selecting
/// (`caret.*`, `select.*`).
const KEYS: &[&str] = &[
    "text.newParagraph",
    "text.lineBreak",
    "text.tab",
    "text.backTab",
    "text.backspace",
    "text.delete",
    "text.deleteWordBack",
    "text.deleteSentenceBack",
    "text.deleteWordForward",
    "text.nbsp",
    "edit.undo",
    "edit.redo",
    "edit.cut",
    "edit.copy",
    "edit.paste",
    "edit.pasteText",
    "format.bold",
    "format.italic",
    "format.underline",
    "para.bullets",
    "para.numbering",
    "file.save",
];

/// Whether a key bound to the command `id` runs it in simple editing.
pub fn key_runs(id: &str) -> bool {
    id.starts_with("caret.") || id.starts_with("select.") || KEYS.contains(&id)
}

/// The paste a person's paste becomes in simple editing: every paste keeps the text only. Other
/// commands are left as they are.
pub fn paste_as(id: &str) -> &str {
    match id {
        "edit.paste" | "edit.pasteMerge" | "edit.pasteSpecial" => "edit.pasteText",
        other => other,
    }
}

/// The context menu's items in simple editing: (label, command).
pub const MENU: [(&str, &str); 3] = [("Cut", "edit.cut"), ("Copy", "edit.copy"), ("Paste", "edit.paste")];

/// The row of buttons above the page.
pub fn toolbar(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let st = app.session.run("format.state", &json!({})).unwrap_or_default();
    let flag = |k: &str| st.get(k).and_then(serde_json::Value::as_bool).unwrap_or(false);
    let rp = app.session.doc.para_at(&app.session.sel.focus).map(|_| wordcraft_engine::cmd::para::selection_para(&app.session));
    let lists = wordcraft_engine::cmd::para::list_buttons(&app.session.doc, rp.as_ref().and_then(|r| r.numbering));
    let checked = |id: &str| match id {
        "format.bold" => flag("bold"),
        "format.italic" => flag("italic"),
        "format.underline" => flag("underline"),
        "para.bullets" => lists.bullets,
        "para.numbering" => lists.numbering,
        _ => false,
    };
    egui::Panel::top("simple_toolbar").frame(egui::Frame::NONE.fill(t.ribbon).inner_margin(egui::Margin::symmetric(8, 4))).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
            for (i, (icon, tip, id)) in BUTTONS.into_iter().enumerate() {
                // Undo and Redo, the character formats, the lists: a gap between the groups.
                if i == 2 || i == 5 {
                    ui.add_space(8.0);
                }
                crate::widgets::small(ui, app, icon, None, tip, crate::widgets::ButtonAction::new(id, json!({}), checked(id)));
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    use wordcraft_doc::{Pos, StoryRef};
    use wordcraft_engine::Session;

    /// A window on a document with "Hello" typed in it; `simple` is what a host sets first.
    fn harness(simple: bool) -> egui_kittest::Harness<'static, WordApp> {
        let mut a = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
        a.run("text.insert", json!({"text": "Hello"})).unwrap();
        a.simple_edit = simple;
        let mut h = egui_kittest::Harness::builder().with_size(egui::vec2(1200.0, 800.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            a,
        );
        h.run_steps(6);
        h
    }

    fn bold(a: &mut WordApp) -> bool {
        a.session.run("format.state", &json!({})).unwrap()["bold"].as_bool().unwrap_or(false)
    }

    #[test]
    fn a_simple_window_shows_the_row_of_buttons_and_no_word_processor_chrome() {
        let full = harness(false);
        let h = harness(true);
        // The title bar's Account, the ribbon's File tab, the status bar's Zoom.
        for chrome in ["Account", "File", "Zoom"] {
            assert!(full.query_by_label(chrome).is_some(), "the full editor draws {chrome}");
            assert!(h.query_by_label(chrome).is_none(), "a simple one does not draw {chrome}");
        }
        for (_, tip, _) in BUTTONS {
            assert!(h.query_by_label(tip).is_some(), "the row offers {tip}");
        }
        let (full_page, simple_page) = (full.state().canvas.page_rects[0], h.state().canvas.page_rects[0]);
        assert!(simple_page.min.y < full_page.min.y, "the page moves up into the room the chrome took");
        // The rulers take the strip left of the page in the full editor: without them the page
        // sits centred in the whole width.
        assert!(full.state().session.view.ruler, "the full editor shows its rulers");
        let (full_gap, simple_gap) = (full_page.min.x, simple_page.min.x);
        let width = |h: &egui_kittest::Harness<'static, WordApp>| h.state().canvas.page_rects[0].width();
        assert!((width(&full) - width(&h)).abs() < 0.5, "the same page");
        assert!(simple_gap < full_gap, "no vertical ruler beside the page: {simple_gap} vs {full_gap}");
    }

    #[test]
    fn the_rows_buttons_format_and_follow_the_editing_restrictions() {
        let mut h = harness(true);
        h.state_mut().run("select.all", json!({})).unwrap();
        h.run_steps(2);
        h.get_by_label("Bold").click();
        h.run_steps(3);
        assert!(bold(h.state_mut()), "Bold bolds the selection");
        h.get_by_label("Undo").click();
        h.run_steps(3);
        assert!(!bold(h.state_mut()), "Undo takes it back");
        h.state_mut().execute("review.restrict", json!({"mode": "readOnly"})).unwrap();
        h.run_steps(3);
        for tip in ["Bold", "Italic", "Underline", "Bullets", "Numbering"] {
            assert!(h.get_by_label(tip).accesskit_node().is_disabled(), "{tip} is greyed out in a read-only document");
        }
        h.get_by_label("Bold").click();
        h.run_steps(3);
        assert!(!bold(h.state_mut()), "and a click does nothing");
    }

    #[test]
    fn every_paste_in_a_simple_window_keeps_the_text_only() {
        for simple in [false, true] {
            let mut a = WordApp::new(Session::new(wordcraft_doc::Document::new()), crate::Services::default());
            a.simple_edit = simple;
            for id in ["edit.paste", "edit.pasteMerge"] {
                a.run(id, json!({"text": "Strong", "html": "<p><b>Strong</b></p>"})).unwrap();
            }
            let body = a.session.doc.plain_text(StoryRef::Body);
            assert!(body.contains("StrongStrong"), "{body:?}");
            let para = a.session.doc.para_at(&Pos::body(0, 0)).unwrap();
            assert_eq!(para.props_at(1).bold == Some(true), !simple, "simple: {simple}: the copy's bold comes in only in the full editor");
        }
    }

    #[test]
    fn keys_keep_typing_moving_and_the_rows_own_commands() {
        for id in ["caret.left", "caret.docEnd", "select.all", "text.newParagraph", "text.backspace", "edit.undo", "edit.copy", "format.bold"] {
            assert!(key_runs(id), "{id} runs");
        }
        for (_, _, id) in BUTTONS {
            assert!(key_runs(id), "{id}, on the row, also runs from its key");
        }
    }

    #[test]
    fn keys_for_other_formats_inserting_and_files_do_nothing() {
        for id in [
            "format.strikethrough",
            "format.fontDialog",
            "format.growFont",
            "para.alignCenter",
            "para.heading1",
            "text.pageBreak",
            "edit.find",
            "edit.repeat",
            "edit.pasteFormat",
            "file.open",
            "file.print",
            "insert.link",
            "review.newComment",
        ] {
            assert!(!key_runs(id), "{id} does nothing");
        }
    }

    #[test]
    fn every_paste_keeps_the_text_only() {
        for id in ["edit.paste", "edit.pasteMerge", "edit.pasteSpecial", "edit.pasteText"] {
            assert_eq!(paste_as(id), "edit.pasteText");
        }
        assert_eq!(paste_as("edit.copy"), "edit.copy");
    }
}
