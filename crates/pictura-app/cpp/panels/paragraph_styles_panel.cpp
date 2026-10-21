#include "paragraph_styles_panel.h"

#include "dialogs.h"
#include "icons.h"
#include "paragraph_style_dialog.h"
#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "api/type_api.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QListWidget>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QToolButton* footerButton(QWidget* parent, const char* iconId, const QString& tip, const char* name)
{
    auto* button = new QToolButton(parent);
    button->setObjectName(QLatin1String(name));
    button->setIcon(icon(QLatin1String(iconId)));
    button->setToolTip(tip);
    button->setAutoRaise(true);
    return button;
}

} // namespace

ParagraphStylesPanel::ParagraphStylesPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);

    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("paragraphStylesList"));
    list_->setSelectionMode(QAbstractItemView::SingleSelection);
    root->addWidget(list_, 1);

    connect(list_, &QListWidget::itemClicked, this, [this](QListWidgetItem* item) {
        if (!updating_) {
            applyRow(list_->row(item));
        }
    });
    connect(list_, &QListWidget::itemDoubleClicked, this, [this](QListWidgetItem* item) {
        if (!updating_) {
            editStyle(list_->row(item));
        }
    });

    auto* footer = new QWidget(this);
    footer->setObjectName(QStringLiteral("panelFooter"));
    auto* row = new QHBoxLayout(footer);
    row->setContentsMargins(4, 2, 4, 2);
    row->setSpacing(2);
    row->addStretch(1);

    add_ = footerButton(footer, "select.mode.add",
                        tr("Create a new paragraph style"), "paragraphStyleAdd");
    connect(add_, &QToolButton::clicked, this, &ParagraphStylesPanel::createStyle);
    row->addWidget(add_);

    remove_ = footerButton(footer, "path.delete", tr("Delete the selected paragraph style"),
                           "paragraphStyleDelete");
    connect(remove_, &QToolButton::clicked, this,
            &ParagraphStylesPanel::deleteSelectedStyle);
    row->addWidget(remove_);

    root->addWidget(footer);
    refresh();
}

void ParagraphStylesPanel::setView(PictureView* view)
{
    view_ = view;
    refresh();
}

void ParagraphStylesPanel::refresh()
{
    const QSignalBlocker blocker(list_);
    updating_ = true;
    const int current = list_->currentRow();
    QStringList names;
    if (view_ && view_->has_document()) {
        const int count = pictura::type_api::paragraphStyleCount(*view_);
        names.reserve(count);
        for (int i = 0; i < count; ++i) {
            names << pictura::type_api::paragraphStyleName(*view_, i);
        }
    }
    // Applying a style records history, and the frame's debounced panel
    // refresh then lands mid-double-click; rebuilding identical rows would
    // destroy the pressed item and swallow the double-click. Only rebuild when
    // the style list actually changed.
    bool same = names.size() == list_->count();
    for (int i = 0; same && i < names.size(); ++i) {
        same = list_->item(i)->text() == names.at(i);
    }
    if (!same) {
        list_->clear();
        for (int i = 0; i < names.size(); ++i) {
            auto* item = new QListWidgetItem(names.at(i), list_);
            item->setData(Qt::UserRole, i);
        }
    }
    updating_ = false;
    if (current >= 0 && current < list_->count()) {
        list_->setCurrentRow(current);
    }
    remove_->setEnabled(list_->count() > 1);
}

int ParagraphStylesPanel::selectedRow() const { return list_->currentRow(); }

void ParagraphStylesPanel::applyRow(int row)
{
    if (!view_ || !view_->has_document() || row < 0) {
        return;
    }
    QListWidgetItem* item = list_->item(row);
    if (!item) {
        return;
    }
    const QString name = item->text();
    const QString path = layer_selection_anchor(*view_);
    if (path.isEmpty() || !pictura::type_api::isType(*view_, path)) {
        return;
    }
    pictura::type_api::applyStyle(*view_, path, name, true);
}

bool ParagraphStylesPanel::createStyle()
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    const QString suggested = tr("Paragraph Style %1").arg(list_->count());
    // A new style starts from the document defaults. Nothing applies it yet, so
    // the dialog's Preview checkbox is disabled.
    const CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    const ParagraphSetting paragraph = pictura::type_api::defaultParagraphSetting();

    ParagraphStyleDialog dialog(suggested, QString(), character, paragraph, false, this);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return false;
    }

    const QString family = dialog.family();
    registerTypeFont(family);
    const bool created = pictura::type_api::createParagraphStyle(*view_, dialog.styleName(), family,
                                                     dialog.characterSetting(),
                                                     dialog.paragraphSetting());
    if (created) {
        refresh();
        list_->setCurrentRow(list_->count() - 1);
    }
    return created;
}

bool ParagraphStylesPanel::editStyle(int row)
{
    if (!view_ || !view_->has_document() || row < 0 || row >= list_->count()) {
        return false;
    }
    const QString name = list_->item(row)->text();
    const CharacterSetting originalCharacter = pictura::type_api::paragraphStyleCharacter(*view_, name);
    const ParagraphSetting originalParagraph = pictura::type_api::paragraphStyleParagraph(*view_, name);
    const QString family = familyForFontName(pictura::type_api::paragraphStyleFont(*view_, name));

    ParagraphStyleDialog dialog(name, family, originalCharacter, originalParagraph, true, this);
    bool previewed = false;
    connect(&dialog, &ParagraphStyleDialog::previewChanged, this,
            [this, &dialog, &name, &previewed] {
                previewed = true;
                pictura::type_api::previewParagraphStyle(*view_, name, dialog.family(),
                                             dialog.characterSetting(),
                                             dialog.paragraphSetting());
            });
    if (runDialog(dialog, this) != QDialog::Accepted) {
        if (previewed) {
            pictura::type_api::previewParagraphStyle(*view_, name, family, originalCharacter,
                                         originalParagraph);
        }
        return false;
    }

    const QString newFamily = dialog.family();
    registerTypeFont(newFamily);
    // The bridge edits by current name; a rename deletes and recreates.
    const QString newName = dialog.styleName();
    if (newName == name) {
        pictura::type_api::editParagraphStyle(*view_, name, newFamily, dialog.characterSetting(),
                                  dialog.paragraphSetting());
    } else {
        pictura::type_api::createParagraphStyle(*view_, newName, newFamily, dialog.characterSetting(),
                                    dialog.paragraphSetting());
        pictura::type_api::deleteParagraphStyle(*view_, name);
    }
    refresh();
    return true;
}

bool ParagraphStylesPanel::deleteSelectedStyle()
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    const int row = selectedRow();
    // Row 0 is the Basic Paragraph style every document has; it cannot be
    // deleted.
    if (row <= 0 || row >= list_->count()) {
        return false;
    }
    const QString name = list_->item(row)->text();
    const bool deleted = pictura::type_api::deleteParagraphStyle(*view_, name);
    if (deleted) {
        refresh();
        list_->setCurrentRow(qMin(row, list_->count() - 1));
    }
    return deleted;
}

} // namespace pictura

#include "moc_paragraph_styles_panel.cpp"
