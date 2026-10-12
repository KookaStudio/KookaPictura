#include "clone_source_panel.h"

#include "numeric_field.h"

#include "icons.h"
#include "tools.h"

#include <QtCore/QSignalBlocker>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QScrollArea>
#include <QtWidgets/QVBoxLayout>


namespace pictura {

namespace {

NumericFieldConfig field(double lo, double hi, int decimals, const QString& suffix,
                         const QString& name)
{
    NumericFieldConfig config;
    config.minimum = lo;
    config.maximum = hi;
    config.decimals = decimals;
    config.step = decimals ? 0.1 : 1.0;
    config.suffix = suffix;
    config.objectName = name;
    return config;
}

QFrame* separator(QWidget* parent)
{
    auto* line = new QFrame(parent);
    line->setFrameShape(QFrame::HLine);
    line->setFrameShadow(QFrame::Sunken);
    return line;
}

QToolButton* toggle(QWidget* parent, const QString& text, const QString& tip, const QString& name)
{
    auto* button = new QToolButton(parent);
    button->setObjectName(name);
    button->setText(text);
    button->setToolTip(tip);
    button->setCheckable(true);
    button->setAutoRaise(true);
    return button;
}

} // namespace

CloneSourcePanel::CloneSourcePanel(QWidget* parent)
    : QWidget(parent)
{
    // The content scrolls, so the panel's size never widens or lengthens the
    // column and groups it shares.
    auto* outer = new QVBoxLayout(this);
    outer->setContentsMargins(0, 0, 0, 0);
    auto* scroll = new QScrollArea(this);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    auto* content = new QWidget(scroll);
    scroll->setWidget(content);
    outer->addWidget(scroll);
    auto* layout = new QVBoxLayout(content);

    auto* slotRow = new QHBoxLayout();
    auto* group = new QButtonGroup(this);
    group->setExclusive(true);
    slotRow->addStretch(1);
    for (int i = 0; i < int(slots_.size()); ++i) {
        auto* button = toggle(this, QString::number(i + 1), tr("Clone Source %1").arg(i + 1),
                              QStringLiteral("cloneSourceSlot%1").arg(i + 1));
        button->setIcon(pictura::icon(QStringLiteral("tool.clonestamp")));
        button->setToolButtonStyle(Qt::ToolButtonTextUnderIcon);
        group->addButton(button, i);
        slotRow->addWidget(button);
        slots_[i] = button;
    }
    slotRow->addStretch(1);
    layout->addLayout(slotRow);
    layout->addWidget(separator(this));

    auto* grid = new QGridLayout();
    grid->addWidget(new QLabel(tr("Offset:"), this), 0, 0, 1, 2);
    offsetX_ = new NumericField(tr("X:"), field(-30000, 30000, 0, tr(" px"),
                                                QStringLiteral("cloneSourceOffsetX")),
                                this);
    offsetY_ = new NumericField(tr("Y:"), field(-30000, 30000, 0, tr(" px"),
                                                QStringLiteral("cloneSourceOffsetY")),
                                this);
    grid->addWidget(offsetX_, 1, 0, 1, 2);
    grid->addWidget(offsetY_, 2, 0, 1, 2);

    flipH_ = toggle(this, QStringLiteral("⇋"), tr("Flip Horizontal"),
                    QStringLiteral("cloneSourceFlipH"));
    flipV_ = toggle(this, QStringLiteral("⇵"), tr("Flip Vertical"),
                    QStringLiteral("cloneSourceFlipV"));
    width_ = new NumericField(tr("W:"), field(1, 1000, 1, QStringLiteral("%"),
                                              QStringLiteral("cloneSourceWidth")),
                              this);
    height_ = new NumericField(tr("H:"), field(1, 1000, 1, QStringLiteral("%"),
                                               QStringLiteral("cloneSourceHeight")),
                               this);
    link_ = toggle(this, QStringLiteral("⛓"), tr("Maintain Aspect Ratio"),
                   QStringLiteral("cloneSourceLink"));
    link_->setChecked(true);
    angle_ = new NumericField(tr("∠"), field(-360, 360, 1, QStringLiteral("°"),
                                            QStringLiteral("cloneSourceAngle")),
                              this);
    auto* reset = new QToolButton(this);
    reset->setObjectName(QStringLiteral("cloneSourceReset"));
    reset->setText(QStringLiteral("↺"));
    reset->setToolTip(tr("Reset Transform"));
    reset->setAutoRaise(true);
    grid->addWidget(flipH_, 0, 2);
    grid->addWidget(width_, 0, 3);
    grid->addWidget(flipV_, 1, 2);
    grid->addWidget(height_, 1, 3);
    grid->addWidget(link_, 0, 4, 2, 1);
    grid->addWidget(angle_, 2, 3);
    grid->addWidget(reset, 2, 4);
    layout->addLayout(grid);
    layout->addWidget(separator(this));

    auto* frameRow = new QHBoxLayout();
    auto* frameOffset = new NumericField(
        tr("Frame Offset:"), field(-9999, 9999, 0, QString(), QStringLiteral("cloneSourceFrame")),
        this);
    frameOffset->setEnabled(false);
    auto* lockFrame = new QCheckBox(tr("Lock Frame"), this);
    lockFrame->setEnabled(false);
    frameOffset->setToolTip(tr("Frame Offset (Extended): not implemented yet"));
    lockFrame->setToolTip(tr("Lock Frame (Extended): not implemented yet"));
    frameRow->addWidget(frameOffset);
    frameRow->addWidget(lockFrame);
    layout->addLayout(frameRow);
    layout->addWidget(separator(this));

    auto* overlay = new QGridLayout();
    const QString notYet = tr("Source overlay: not implemented yet");
    auto overlayCheck = [&](const QString& text, bool on, int row, int col) {
        auto* check = new QCheckBox(text, this);
        check->setChecked(on);
        check->setEnabled(false);
        check->setToolTip(notYet);
        overlay->addWidget(check, row, col);
    };
    overlayCheck(tr("Show Overlay"), false, 0, 0);
    overlayCheck(tr("Clipped"), false, 0, 1);
    auto* opacity = new NumericField(tr("Opacity:"), field(0, 100, 0, QStringLiteral("%"),
                                                           QStringLiteral("cloneSourceOverlayOpacity")),
                                     this);
    opacity->setValue(100);
    opacity->setEnabled(false);
    opacity->setToolTip(notYet);
    overlay->addWidget(opacity, 1, 0);
    overlayCheck(tr("Auto Hide"), false, 1, 1);
    auto* mode = new QComboBox(this);
    mode->addItems({tr("Normal"), tr("Darken"), tr("Lighten"), tr("Difference")});
    mode->setEnabled(false);
    mode->setToolTip(notYet);
    overlay->addWidget(mode, 2, 0);
    overlayCheck(tr("Invert"), false, 2, 1);
    layout->addLayout(overlay);
    layout->addStretch(1);

    connect(group, &QButtonGroup::idClicked, this, [this](int id) {
        if (clone_) {
            clone_->setSlot(id);
        }
    });
    // Offset is shown as destination minus source, the engine keeps the other
    // way round; typing it pins the next stroke's anchor to the source plus it.
    auto setOffset = [this] {
        const QPoint shown(qRound(offsetX_->value()), qRound(offsetY_->value()));
        editSlot([shown](CloneSource& s) {
            s.offset = -shown;
            s.anchor = s.source + shown;
            s.hasOffset = true;
        });
    };
    connect(offsetX_, &NumericField::valueChanged, this, setOffset);
    connect(offsetY_, &NumericField::valueChanged, this, setOffset);
    connect(width_, &NumericField::valueChanged, this, [this](double v) {
        const bool linked = link_->isChecked();
        editSlot([v, linked](CloneSource& s) {
            s.width = v;
            if (linked) {
                s.height = v;
            }
        });
    });
    connect(height_, &NumericField::valueChanged, this, [this](double v) {
        const bool linked = link_->isChecked();
        editSlot([v, linked](CloneSource& s) {
            s.height = v;
            if (linked) {
                s.width = v;
            }
        });
    });
    connect(angle_, &NumericField::valueChanged, this,
            [this](double v) { editSlot([v](CloneSource& s) { s.angle = v; }); });
    connect(flipH_, &QToolButton::toggled, this,
            [this](bool on) { editSlot([on](CloneSource& s) { s.flipH = on; }); });
    connect(flipV_, &QToolButton::toggled, this,
            [this](bool on) { editSlot([on](CloneSource& s) { s.flipV = on; }); });
    connect(reset, &QToolButton::clicked, this, [this] {
        editSlot([](CloneSource& s) {
            s.width = 100.0;
            s.height = 100.0;
            s.angle = 0.0;
            s.flipH = false;
            s.flipV = false;
        });
    });
}

void CloneSourcePanel::setController(ToolController* controller)
{
    controller_ = controller;
    clone_ = controller ? controller->optionsFor<CloneSourceOptions>(ToolId::CloneStamp) : nullptr;
    if (controller_) {
        connect(controller_, &ToolController::cloneSourceChanged, this, [this] { refresh(); });
    }
    refresh();
}

void CloneSourcePanel::editSlot(const std::function<void(CloneSource&)>& edit)
{
    if (!clone_) {
        return;
    }
    CloneSource slot = clone_->active();
    edit(slot);
    clone_->setActive(slot);
}

void CloneSourcePanel::refresh()
{
    if (!clone_) {
        return;
    }
    const CloneSource s = clone_->active();
    for (int i = 0; i < int(slots_.size()); ++i) {
        slots_[i]->setChecked(i == clone_->slot());
    }
    const QPoint shown = s.hasOffset ? -s.offset : QPoint();
    offsetX_->setValue(shown.x());
    offsetY_->setValue(shown.y());
    // An offset needs a source point to measure from.
    offsetX_->setEnabled(s.hasSource);
    offsetY_->setEnabled(s.hasSource);
    width_->setValue(s.width);
    height_->setValue(s.height);
    angle_->setValue(s.angle);
    const QSignalBlocker bh(flipH_);
    const QSignalBlocker bv(flipV_);
    flipH_->setChecked(s.flipH);
    flipV_->setChecked(s.flipV);
}

} // namespace pictura
