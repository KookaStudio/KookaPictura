#include "paragraph_panel.h"

#include "dialogs.h"
#include "justification_dialog.h"
#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "api/type_api.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QAction>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QMenu>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QDoubleSpinBox* spin(QWidget* parent, const QString& name, double lo, double hi, int decimals)
{
    auto* box = new QDoubleSpinBox(parent);
    box->setObjectName(name);
    box->setRange(lo, hi);
    box->setDecimals(decimals);
    box->setKeyboardTracking(false);
    return box;
}

QAction* findMenuAction(QMenu* menu, const QString& id)
{
    for (QAction* action : menu->actions()) {
        if (action->data().toString() == id) {
            return action;
        }
        if (QMenu* submenu = action->menu()) {
            if (QAction* found = findMenuAction(submenu, id)) {
                return found;
            }
        }
    }
    return nullptr;
}

} // namespace

ParagraphPanel::ParagraphPanel(QWidget* parent)
    : QWidget(parent)
{
    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(4, 4, 4, 4);

    auto* paragraphs = new QGroupBox(tr("Paragraph"), this);
    auto* form = new QFormLayout(paragraphs);
    justify_ = new QComboBox(paragraphs);
    justify_->setObjectName(QStringLiteral("paragraphJustify"));
    justify_->addItems({tr("Left"), tr("Right"), tr("Center"), tr("Justify Last Left"),
                        tr("Justify Last Right"), tr("Justify Last Center"), tr("Justify All")});
    form->addRow(tr("Alignment"), justify_);
    startIndent_ = spin(paragraphs, QStringLiteral("paragraphStartIndent"), -5000.0, 5000.0, 1);
    form->addRow(tr("Left Indent"), startIndent_);
    endIndent_ = spin(paragraphs, QStringLiteral("paragraphEndIndent"), -5000.0, 5000.0, 1);
    form->addRow(tr("Right Indent"), endIndent_);
    firstLineIndent_ =
        spin(paragraphs, QStringLiteral("paragraphFirstLineIndent"), -5000.0, 5000.0, 1);
    form->addRow(tr("First Line Indent"), firstLineIndent_);
    spaceBefore_ = spin(paragraphs, QStringLiteral("paragraphSpaceBefore"), 0.0, 5000.0, 1);
    form->addRow(tr("Space Before"), spaceBefore_);
    spaceAfter_ = spin(paragraphs, QStringLiteral("paragraphSpaceAfter"), 0.0, 5000.0, 1);
    form->addRow(tr("Space After"), spaceAfter_);
    hanging_ = new QCheckBox(tr("Hanging Punctuation"), paragraphs);
    hanging_->setObjectName(QStringLiteral("paragraphHanging"));
    form->addRow(QString(), hanging_);
    hyphenate_ = new QCheckBox(tr("Hyphenate"), paragraphs);
    hyphenate_->setObjectName(QStringLiteral("paragraphHyphenate"));
    form->addRow(QString(), hyphenate_);
    composer_ = new QComboBox(paragraphs);
    composer_->setObjectName(QStringLiteral("paragraphComposer"));
    composer_->addItems({tr("Single-line"), tr("Every-line")});
    form->addRow(tr("Composer"), composer_);
    auto* direction = new QWidget(paragraphs);
    form_ = form;
    direction_ = direction;
    auto* directionRow = new QHBoxLayout(direction);
    directionRow->setContentsMargins(0, 0, 0, 0);
    ltr_ = new QToolButton(direction);
    ltr_->setObjectName(QStringLiteral("paragraphDirectionLtr"));
    ltr_->setText(tr("LTR"));
    ltr_->setCheckable(true);
    ltr_->setToolTip(tr("Left-to-right paragraph direction"));
    rtl_ = new QToolButton(direction);
    rtl_->setObjectName(QStringLiteral("paragraphDirectionRtl"));
    rtl_->setText(tr("RTL"));
    rtl_->setCheckable(true);
    rtl_->setToolTip(tr("Right-to-left paragraph direction"));
    directionRow->addWidget(ltr_);
    directionRow->addWidget(rtl_);
    form->addRow(tr("Direction"), direction);
    layout->addWidget(paragraphs);
    layout->addStretch(1);

    const auto commit = [this](QDoubleSpinBox* box) {
        connect(box, &QAbstractSpinBox::editingFinished, this, &ParagraphPanel::apply);
    };
    commit(startIndent_);
    commit(endIndent_);
    commit(firstLineIndent_);
    commit(spaceBefore_);
    commit(spaceAfter_);
    connect(justify_, &QComboBox::currentIndexChanged, this, &ParagraphPanel::apply);
    connect(composer_, &QComboBox::currentIndexChanged, this, &ParagraphPanel::apply);
    connect(hanging_, &QCheckBox::toggled, this, &ParagraphPanel::apply);
    connect(hyphenate_, &QCheckBox::toggled, this, &ParagraphPanel::apply);
    connect(ltr_, &QToolButton::clicked, this, [this] {
        ltr_->setChecked(true);
        rtl_->setChecked(false);
        apply();
    });
    connect(rtl_, &QToolButton::clicked, this, [this] {
        rtl_->setChecked(true);
        ltr_->setChecked(false);
        apply();
    });

    refresh();
}

bool ParagraphPanel::hasTypeLayer() const
{
    if (!view_ || !view_->has_document()) {
        return false;
    }
    const QString path = layer_selection_anchor(*view_);
    return !path.isEmpty() && pictura::type_api::isType(*view_, path);
}

void ParagraphPanel::setView(PictureView* view)
{
    view_ = view;
    refresh();
}

void ParagraphPanel::setMiddleEastern(bool on)
{
    if (middleEastern_ == on) {
        return;
    }
    middleEastern_ = on;
    refresh();
}

QSize ParagraphPanel::minimumSizeHint() const
{
    return QSize(0, QWidget::minimumSizeHint().height());
}

void ParagraphPanel::refresh()
{
    const bool type = hasTypeLayer();
    const QString path = type ? layer_selection_anchor(*view_) : QString();
    const ParagraphSetting p = type ? pictura::type_api::layerParagraphSetting(*view_, path)
                                    : pictura::type_api::defaultParagraphSetting();

    const QSignalBlocker blockJustify(justify_);
    const QSignalBlocker blockStart(startIndent_);
    const QSignalBlocker blockEnd(endIndent_);
    const QSignalBlocker blockFirst(firstLineIndent_);
    const QSignalBlocker blockBefore(spaceBefore_);
    const QSignalBlocker blockAfter(spaceAfter_);
    const QSignalBlocker blockHanging(hanging_);
    const QSignalBlocker blockHyphenate(hyphenate_);
    const QSignalBlocker blockComposer(composer_);
    const QSignalBlocker blockLtr(ltr_);
    const QSignalBlocker blockRtl(rtl_);

    justify_->setCurrentIndex(qBound(0, p.justify, 6));
    startIndent_->setValue(p.start_indent);
    endIndent_->setValue(p.end_indent);
    firstLineIndent_->setValue(p.first_line_indent);
    spaceBefore_->setValue(p.space_before);
    spaceAfter_->setValue(p.space_after);
    hanging_->setChecked(p.hanging);
    hyphenate_->setChecked(p.hyphenate);
    composer_->setCurrentIndex(p.composer == 1 ? 1 : 0);
    ltr_->setChecked(p.direction == 0);
    rtl_->setChecked(p.direction == 1);
    if (form_ && direction_) {
        form_->setRowVisible(direction_, middleEastern_);
    }

    for (QWidget* widget : QList<QWidget*>{justify_, startIndent_, endIndent_, firstLineIndent_,
                                           spaceBefore_, spaceAfter_, hanging_, hyphenate_,
                                           composer_, ltr_, rtl_}) {
        widget->setEnabled(type);
    }
}

void ParagraphPanel::apply()
{
    if (!hasTypeLayer()) {
        return;
    }
    const QString path = layer_selection_anchor(*view_);
    const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*view_, path);
    ParagraphSetting p = layer.paragraph;
    p.justify = justify_->currentIndex();
    p.start_indent = startIndent_->value();
    p.end_indent = endIndent_->value();
    p.first_line_indent = firstLineIndent_->value();
    p.space_before = spaceBefore_->value();
    p.space_after = spaceAfter_->value();
    p.hanging = hanging_->isChecked();
    p.hyphenate = hyphenate_->isChecked();
    p.composer = composer_->currentIndex() == 1 ? 1 : 0;
    if (middleEastern_) {
        p.direction = rtl_->isChecked() ? 1 : 0;
    }

    const QString family = familyForFontName(layer.font);
    registerTypeFont(family, layer.character.font_style);
    pictura::type_api::updateLayer(*view_, path, layer.text, family, layer.setting,
                                   layer.character, p);
}

void ParagraphPanel::updatePanelMenu(QMenu* menu)
{
    const bool type = hasTypeLayer();
    const auto sync = [menu](const char* id, bool checked, bool enabled) {
        QAction* action = findMenuAction(menu, QString::fromLatin1(id));
        if (!action) {
            return;
        }
        const QSignalBlocker block(action);
        if (action->isCheckable()) {
            action->setChecked(checked);
        }
        action->setEnabled(enabled);
    };
    sync("paragraph.hanging", hanging_->isChecked(), type);
    sync("paragraph.singleLineComposer", composer_->currentIndex() == 0, type);
    sync("paragraph.everyLineComposer", composer_->currentIndex() == 1, type);
    sync("paragraph.justification", false, type);
    sync("paragraph.reset", false, type);
    // The disabled layout/feature rows still show CS6's default choices.
    for (QAction* action : menu->actions()) {
        if (action->text() == QStringLiteral("Latin and East Asian Layout")
            || action->text() == QStringLiteral("Default Features")) {
            const QSignalBlocker block(action);
            action->setChecked(true);
        }
    }
}

void ParagraphPanel::performPanelMenuAction(const QString& actionId)
{
    if (actionId == QLatin1String("paragraph.hanging")) {
        hanging_->setChecked(!hanging_->isChecked());
        return;
    }
    if (actionId == QLatin1String("paragraph.justification")) {
        openJustification();
        return;
    }
    if (actionId == QLatin1String("paragraph.singleLineComposer")) {
        if (composer_->currentIndex() != 0) {
            composer_->setCurrentIndex(0);
        }
        return;
    }
    if (actionId == QLatin1String("paragraph.everyLineComposer")) {
        if (composer_->currentIndex() != 1) {
            composer_->setCurrentIndex(1);
        }
        return;
    }
    if (actionId == QLatin1String("paragraph.reset")) {
        resetParagraph();
    }
}

void ParagraphPanel::resetParagraph()
{
    if (!hasTypeLayer()) {
        return;
    }
    const QString path = layer_selection_anchor(*view_);
    const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*view_, path);
    const ParagraphSetting defaults = pictura::type_api::defaultParagraphSetting();
    const QString family = familyForFontName(layer.font);
    registerTypeFont(family, layer.character.font_style);
    pictura::type_api::updateLayer(*view_, path, layer.text, family, layer.setting,
                                   layer.character, defaults);
    refresh();
}

void ParagraphPanel::openJustification()
{
    if (!hasTypeLayer()) {
        return;
    }
    const QString path = layer_selection_anchor(*view_);
    const pictura::type_api::LayerType layer = pictura::type_api::readLayer(*view_, path);
    JustificationDialog dialog(layer.paragraph, this);
    if (runDialog(dialog, this) != QDialog::Accepted) {
        return;
    }
    const ParagraphSetting p = dialog.setting();
    const QString family = familyForFontName(layer.font);
    registerTypeFont(family, layer.character.font_style);
    pictura::type_api::updateLayer(*view_, path, layer.text, family, layer.setting,
                                   layer.character, p);
}

bool ParagraphPanel::directionVisibleForTest() const
{
    return ltr_->isVisibleTo(this);
}

bool ParagraphPanel::editingEnabledForTest() const { return justify_->isEnabled(); }

void ParagraphPanel::commitForTest() { apply(); }

} // namespace pictura

#include "moc_paragraph_panel.cpp"
