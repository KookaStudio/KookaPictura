#include "paragraph_panel.h"

#include "type_fonts.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/layer_target_bridge.cxxqt.h"
#include "api/type_api.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QFormLayout>
#include <QtWidgets/QGroupBox>
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

    justify_->setCurrentIndex(qBound(0, p.justify, 6));
    startIndent_->setValue(p.start_indent);
    endIndent_->setValue(p.end_indent);
    firstLineIndent_->setValue(p.first_line_indent);
    spaceBefore_->setValue(p.space_before);
    spaceAfter_->setValue(p.space_after);
    hanging_->setChecked(p.hanging);
    hyphenate_->setChecked(p.hyphenate);
    composer_->setCurrentIndex(p.composer == 1 ? 1 : 0);

    for (QWidget* widget : QList<QWidget*>{justify_, startIndent_, endIndent_, firstLineIndent_,
                                           spaceBefore_, spaceAfter_, hanging_, hyphenate_,
                                           composer_}) {
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

    const QString family = familyForFontName(layer.font);
    registerTypeFont(family);
    pictura::type_api::updateLayer(*view_, path, layer.text, family, layer.setting,
                                   layer.character, p);
}

bool ParagraphPanel::editingEnabledForTest() const { return justify_->isEnabled(); }

void ParagraphPanel::commitForTest() { apply(); }

} // namespace pictura

#include "moc_paragraph_panel.cpp"
