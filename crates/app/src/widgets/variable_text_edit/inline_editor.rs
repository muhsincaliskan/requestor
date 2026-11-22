use eframe::egui::{
    pos2, Area, Context, Frame, Id, Key, Modifiers, Order, Pos2, Response, TextEdit, Ui,
};
use requestor_core::variables::VariablePreview;
use requestor_core::workspace::{VariableAssignment, VariableHome};

use super::hints::VariableHints;

const EDITOR_SALT: &str = "inline-variable-editor";
const VALUE_SALT: &str = "inline-variable-value";
const COMMITTED_SALT: &str = "inline-variable-committed";
const EDITOR_WIDTH: f32 = 300.0;
const ANCHOR_GAP: f32 = 4.0;
const SAVE_TEXT: &str = "Save";
const CANCEL_TEXT: &str = "Cancel";
const NEW_PREFIX: &str = "New in";
const ENVIRONMENT_KIND: &str = "Environment";
const COLLECTION_KIND: &str = "Collection";
const FOLDER_KIND: &str = "Folder";
const KIND_SEPARATOR: &str = " · ";

#[derive(Clone)]
struct InlineEdit {
    field: Id,
    anchor: Pos2,
    name: String,
    value: String,
    secret: bool,
    defined: bool,
    home: VariableHome,
    fresh: bool,
    drawn_in: u64,
}

enum Outcome {
    Keep,
    Save,
    Cancel,
}

pub fn open_on_double_click(
    ctx: &Context,
    field: Id,
    response: &Response,
    name: Option<&str>,
    hints: &VariableHints,
) {
    if !response.double_clicked() {
        return;
    }
    let Some((name, home)) = name.and_then(|name| Some((name, hints.home_of(name)?))) else {
        return;
    };
    let Some(pointer) = response.interact_pointer_pos() else {
        return;
    };
    let preview = hints.preview(name);
    let edit = InlineEdit {
        field,
        anchor: pos2(pointer.x, response.rect.bottom() + ANCHOR_GAP),
        name: name.to_string(),
        value: hints.stored_value(name).unwrap_or_default().to_string(),
        secret: preview == VariablePreview::Secret,
        defined: preview != VariablePreview::Undefined,
        home: home.clone(),
        fresh: true,
        drawn_in: ctx.cumulative_pass_nr(),
    };
    ctx.data_mut(|data| data.insert_temp(Id::new(EDITOR_SALT), edit));
}

pub fn show(ctx: &Context, field: Id) {
    let id = Id::new(EDITOR_SALT);
    let Some(mut edit) = ctx.data(|data| data.get_temp::<InlineEdit>(id)) else {
        return;
    };
    if edit.field != field {
        return;
    }
    match draw(ctx, &mut edit) {
        Outcome::Keep => {
            edit.fresh = false;
            edit.drawn_in = ctx.cumulative_pass_nr();
            ctx.data_mut(|data| data.insert_temp(id, edit));
        }
        Outcome::Save => {
            ctx.data_mut(|data| {
                data.remove::<InlineEdit>(id);
                data.get_temp_mut_or_default::<Vec<VariableAssignment>>(Id::new(COMMITTED_SALT))
                    .push(VariableAssignment {
                        home: edit.home,
                        name: edit.name,
                        value: edit.value,
                    });
            });
        }
        Outcome::Cancel => ctx.data_mut(|data| data.remove::<InlineEdit>(id)),
    }
}

pub fn take_committed(ctx: &Context) -> Vec<VariableAssignment> {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|data| {
        let editor = Id::new(EDITOR_SALT);
        let hidden = data
            .get_temp::<InlineEdit>(editor)
            .is_some_and(|edit| edit.drawn_in != pass);
        if hidden {
            data.remove::<InlineEdit>(editor);
        }
        data.remove_temp(Id::new(COMMITTED_SALT))
            .unwrap_or_default()
    })
}

fn draw(ctx: &Context, edit: &mut InlineEdit) -> Outcome {
    let area = Area::new(Id::new(EDITOR_SALT))
        .order(Order::Foreground)
        .fixed_pos(edit.anchor)
        .show(ctx, |ui| {
            Frame::popup(ui.style())
                .show(ui, |ui| contents(ui, edit))
                .inner
        });
    let dismissed = !edit.fresh && area.response.clicked_elsewhere();
    let escaped = ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
    match area.inner {
        Outcome::Keep if dismissed || escaped => Outcome::Cancel,
        outcome => outcome,
    }
}

fn contents(ui: &mut Ui, edit: &mut InlineEdit) -> Outcome {
    ui.set_width(EDITOR_WIDTH);
    ui.horizontal(|ui| {
        ui.strong(&edit.name);
        ui.weak(home_label(&edit.home, edit.defined));
    });
    let field = ui.add(
        TextEdit::singleline(&mut edit.value)
            .id(Id::new(VALUE_SALT))
            .password(edit.secret)
            .desired_width(f32::INFINITY),
    );
    if edit.fresh {
        field.request_focus();
    }
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter));
    let (saved, cancelled) = ui
        .horizontal(|ui| {
            (
                ui.button(SAVE_TEXT).clicked(),
                ui.button(CANCEL_TEXT).clicked(),
            )
        })
        .inner;
    match (entered || saved, cancelled) {
        (true, _) => Outcome::Save,
        (false, true) => Outcome::Cancel,
        (false, false) => Outcome::Keep,
    }
}

fn home_label(home: &VariableHome, defined: bool) -> String {
    let (kind, owner) = match home {
        VariableHome::Environment { name, .. } => (ENVIRONMENT_KIND, name),
        VariableHome::Collection { name, .. } => (COLLECTION_KIND, name),
        VariableHome::Folder { name, .. } => (FOLDER_KIND, name),
    };
    let place = format!("{kind}{KIND_SEPARATOR}{owner}");
    match defined {
        true => place,
        false => format!("{NEW_PREFIX} {place}"),
    }
}
