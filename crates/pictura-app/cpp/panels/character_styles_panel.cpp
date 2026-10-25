#include "character_styles_panel.h"

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

CharacterStylesPanel::CharacterStylesPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* root = new QVBoxLayout(this);
    root->setContentsMargins(0, 0, 0, 0);
    root->setSpacing(0);

    list_ = new QListWidget(this);
    list_->setObjectName(QStringLiteral("characterStylesList"));
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

    add_ = footerButton(footer, "select.mode.add", tr("Create a new character style"),
                        "characterStyleAdd");
    connect(add_, &QToolButton::clicked, this, &CharacterStylesPanel::createStyle);
    row->addWidget(add_);

    remove_ = footerButton(footer, "path.delete", tr("Delete the selected character style"),
                           "characterStyleDelete");
    connect(remove_, &QToolButton::clicked, this, &CharacterStylesPanel::deleteSelectedStyle);
    connect(list_, &QListWidget::currentRowChanged, this, [this](int row) {
        remove_->setEnabled(row >= 0 && list_->count() > 0);
    });
    row->addWidget(remove_);

    root->addWidget(footer);
    refresh();
}

void CharacterStylesPanel::setView(PictureView* view)
{
    view_ = view;
    refresh();
}

void CharacterStylesPanel::refresh()
{
    const QSignalBlocker blocker(list_);
    updating_ = true;
    const int current = list_->currentRow();
    QStringList names;
    if (view_ && view_->has_document()) {
        const int count = pictura::type_api::characterStyleCount(*view_);
        names.reserve(count);
        for (int i = 0; i < count; ++i) {
            names << pictura::type_api::characterStyleName(*view_, i);
        }
    }
    // Only rebuild when the list changed, so a refresh does not swallow the
    // double-click that opens the dialog.
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
    remove_->setEnabled(list_->count() > 0 && list_->currentRow() >= 0);
}

int CharacterStylesPanel::selectedRow() const { return list_->currentRow(); }

void CharacterStylesPanel::applyRow(int row)
{
    if (!view_ || !view_->has_document() || row < 0) {
        return;
    }
    QListWidgetItem* item = list_->item(row);
    if (!item) {
        return;
    }
    const QString path = layer_selection_anchor(*view_);
    if (path.isEmpty() || !pictura::type_api::isType(*view_, path)) {
        return;
    }
    pictura::type_api::applyStyle(*view_, path, item->text(), false);
}

bool CharacterStylesPanel::createStyle()
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    const QString suggested = tr("Character Style %1").arg(list_->count());
    const CharacterSetting character = pictura::type_api::defaultCharacterSetting();
    const ParagraphSetting paragraph = pictura::type_api::defaultParagraphSetting();

    ParagraphStyleDialog dialog(suggested, QString(), character, paragraph, false, this, true);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return false;
    }

    const QString family = dialog.family();
    registerTypeFont(family, dialog.characterSetting().font_style);
    const bool created = pictura::type_api::createCharacterStyle(
        *view_, dialog.styleName(), family, dialog.characterSetting());
    if (created) {
        refresh();
        list_->setCurrentRow(list_->count() - 1);
        remove_->setEnabled(true);
    }
    return created;
}

bool CharacterStylesPanel::editStyle(int row)
{
    if (!view_ || !view_->has_document() || row < 0 || row >= list_->count()) {
        return false;
    }
    const QString name = list_->item(row)->text();
    const CharacterSetting originalCharacter =
        pictura::type_api::characterStyleCharacter(*view_, name);
    const QString family =
        familyForFontName(pictura::type_api::characterStyleFont(*view_, name));

    ParagraphStyleDialog dialog(name, family, originalCharacter,
                                pictura::type_api::defaultParagraphSetting(), true, this, true);
    bool previewed = false;
    connect(&dialog, &ParagraphStyleDialog::previewChanged, this,
            [this, &dialog, &name, &previewed] {
                previewed = true;
                const QString previewFamily = dialog.family();
                registerTypeFont(previewFamily, dialog.characterSetting().font_style);
                pictura::type_api::previewCharacterStyle(*view_, name, previewFamily,
                                                         dialog.characterSetting());
            });
    if (runDialog(dialog, this) != QDialog::Accepted) {
        if (previewed) {
            registerTypeFont(family, originalCharacter.font_style);
            pictura::type_api::previewCharacterStyle(*view_, name, family, originalCharacter);
        }
        return false;
    }

    const QString newFamily = dialog.family();
    registerTypeFont(newFamily, dialog.characterSetting().font_style);
    const bool edited = pictura::type_api::editCharacterStyle(
        *view_, name, dialog.styleName(), newFamily, dialog.characterSetting());
    if (edited) {
        refresh();
    }
    return edited;
}

bool CharacterStylesPanel::deleteSelectedStyle()
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    const int row = selectedRow();
    if (row < 0 || row >= list_->count()) {
        return false;
    }
    const QString name = list_->item(row)->text();
    const bool deleted = pictura::type_api::deleteCharacterStyle(*view_, name);
    if (deleted) {
        refresh();
        list_->setCurrentRow(qMin(row, list_->count() - 1));
    }
    return deleted;
}

} // namespace pictura

#include "moc_character_styles_panel.cpp"
