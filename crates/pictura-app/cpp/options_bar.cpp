#include "options_bar.h"

#include "icons.h"
#include "color_picker_dialog.h"
#include "panels/numeric_field.h"

#include "pictura_app/src/cxxqt_object.cxxqt.h"
#include "pictura_app/src/cxxqt_object/annotations.cxxqt.h"

#include <QtCore/QSignalBlocker>
#include <QtGui/QIcon>
#include <QtGui/QAction>
#include <QtGui/QActionGroup>
#include <QtGui/QColor>
#include <QtGui/QPixmap>
#include <QtGui/QResizeEvent>
#include <QtWidgets/QButtonGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QFrame>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QInputDialog>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

namespace {

struct ModeButton {
    SelectionMode mode;
    const char* label;
    const char* icon;
};

const ModeButton kModes[] = {
    {SelectionMode::New, "New selection", "select.mode.new"},
    {SelectionMode::Add, "Add to selection", "select.mode.add"},
    {SelectionMode::Subtract, "Subtract from selection", "select.mode.subtract"},
    {SelectionMode::Intersect, "Intersect with selection", "select.mode.intersect"},
};

// The vertical rule drawn after the tool icon on every page (D21).
QFrame* toolSeparator(QWidget* parent)
{
    auto* line = new QFrame(parent);
    line->setObjectName(QStringLiteral("optionsToolSeparator"));
    line->setFrameShape(QFrame::VLine);
    line->setFrameShadow(QFrame::Sunken);
    line->setFixedHeight(18);
    line->setAttribute(Qt::WA_TransparentForMouseEvents);
    return line;
}

} // namespace

NumericFieldConfig OptionsBar::numericConfig(double lo, double hi, double step, int decimals,
                                             const QString& suffix, bool popup,
                                             const QString& name, bool allowFractional)
{
    NumericFieldConfig config;
    config.minimum = lo;
    config.maximum = hi;
    config.step = step;
    config.decimals = decimals;
    config.suffix = suffix;
    config.popup = popup;
    config.objectName = name;
    config.allowFractional = allowFractional;
    return config;
}

OptionsBar::OptionsBar(ToolController* controller, QWidget* parent)
    : QToolBar(QStringLiteral("Options"), parent)
    , controller_(controller)
{
    setObjectName(QStringLiteral("optionsBar"));

    stack_ = new QStackedWidget(this);
    // Page order follows allToolIds(), which matches ToolId's integer order, so
    // the stack index is the enum value.
    for (ToolId id : allToolIds()) {
        stack_->addWidget(buildPage(id));
    }
    addWidget(stack_);

    // The workspace switcher is a directly-positioned child pinned to the right
    // edge (resizeEvent), not a toolbar item, so a wide tool page can never push
    // it into the overflow `»`. The stack reserves right-side content margin so
    // no tool page renders underneath it.
    workspaceSwitcher_ = new QToolButton(this);
    workspaceSwitcher_->setObjectName(QStringLiteral("workspaceSwitcher"));
    workspaceSwitcher_->setPopupMode(QToolButton::MenuButtonPopup);
    workspaceSwitcher_->setToolButtonStyle(Qt::ToolButtonTextOnly);
    workspaceSwitcher_->setAutoRaise(true);
    workspaceSwitcher_->setFocusPolicy(Qt::NoFocus);
    connect(workspaceSwitcher_, &QToolButton::clicked, workspaceSwitcher_,
            [this]() { workspaceSwitcher_->showMenu(); });

    if (controller_) {
        showTool(controller_->activeTool());
    }
}

void OptionsBar::resizeEvent(QResizeEvent* event)
{
    QToolBar::resizeEvent(event);
    if (!workspaceSwitcher_) {
        return;
    }
    constexpr int kMargin = 4;
    const QSize hint = workspaceSwitcher_->sizeHint();
    const int switcherWidth = qMin(hint.width(), qMax(0, width() - 2 * kMargin));
    workspaceSwitcher_->setGeometry(width() - switcherWidth - kMargin,
                                    (height() - hint.height()) / 2, switcherWidth,
                                    hint.height());
    if (stack_) {
        const int reserve = qMax(0, width() - switcherWidth - 2 * kMargin);
        if (stack_->maximumWidth() != reserve) {
            stack_->setMaximumWidth(reserve);
        }
    }
}

void OptionsBar::setWorkspaceMenu(QMenu* menu)
{
    if (workspaceSwitcher_) {
        workspaceSwitcher_->setMenu(menu);
    }
}

void OptionsBar::setActiveWorkspace(const QString& name)
{
    if (workspaceSwitcher_) {
        workspaceSwitcher_->setText(name);
    }
}

QWidget* OptionsBar::buildPage(ToolId id)
{
    QWidget* page = buildPageBody(id);
    if (page) {
        if (auto* layout = qobject_cast<QHBoxLayout*>(page->layout())) {
            layout->insertWidget(1, toolSeparator(page));
        }
    }
    return page;
}

QWidget* OptionsBar::buildPageBody(ToolId id)
{
    switch (id) {
    case ToolId::Lasso:
    case ToolId::PolygonalLasso:
    case ToolId::MagneticLasso:
    case ToolId::Marquee:
    case ToolId::EllipticalMarquee:
        return buildSelectionPage(id);
    case ToolId::MagicWand:
        return buildWandPage(id);
    case ToolId::Crop:
        return buildCropPage(id);
    case ToolId::Eyedropper:
        return buildEyedropperPage(id);
    case ToolId::ColorSampler:
    case ToolId::Ruler:
    case ToolId::Note:
        return buildAnnotationPage(id);
    case ToolId::Count:
        return buildCountPage(id);
    case ToolId::QuickSelection:
        return buildCombinePage(id, true);
    case ToolId::Brush:
    case ToolId::Pencil:
        return buildPaintPage(id);
    case ToolId::SpotHealingBrush:
    case ToolId::HealingBrush:
        return buildHealingPage(id);
    case ToolId::Patch:
        return buildPatchPage(id);
    case ToolId::ContentAwareMove:
        return buildContentAwareMovePage(id);
    case ToolId::RedEye:
        return buildRedEyePage(id);
    case ToolId::ColorReplacement:
        return buildColorReplacementPage(id);
    case ToolId::MixerBrush:
        return buildMixerBrushPage(id);
    case ToolId::CloneStamp:
    case ToolId::PatternStamp:
        return buildStampPage(id);
    case ToolId::HistoryBrush:
        return buildHistoryBrushPage(id);
    case ToolId::ArtHistoryBrush:
        return buildArtHistoryBrushPage(id);
    case ToolId::Eraser:
        return buildEraserPage(id);
    case ToolId::BackgroundEraser:
        return buildBackgroundEraserPage(id);
    case ToolId::MagicEraser:
        return buildMagicEraserPage(id);
    case ToolId::Blur:
    case ToolId::Sharpen:
    case ToolId::Smudge:
        return buildRetouchPage(id);
    case ToolId::Dodge:
    case ToolId::Burn:
    case ToolId::Sponge:
        return buildTonePage(id);
    case ToolId::Gradient:
        return buildGradientPage(id);
    case ToolId::PaintBucket:
        return buildPaintBucketPage(id);
    case ToolId::Pen:
    case ToolId::FreeformPen:
    case ToolId::PathSelection:
        return buildPenPage(id);
    case ToolId::RotateView:
        return buildRotateViewPage(id);
    case ToolId::Move:
        return buildMovePage(id);
    case ToolId::Rectangle:
    case ToolId::RoundedRectangle:
    case ToolId::Ellipse:
    case ToolId::Polygon:
    case ToolId::Line:
    case ToolId::CustomShape:
        return buildShapePage(id);
    case ToolId::HorizontalType:
    case ToolId::VerticalType:
    case ToolId::HorizontalTypeMask:
    case ToolId::VerticalTypeMask:
        return buildTypePage(id);
    default: {
        auto* page = new QWidget(stack_);
        auto* layout = new QHBoxLayout(page);
        layout->setContentsMargins(4, 2, 4, 2);
        layout->addWidget(toolButton(id, page));
        layout->addStretch(1);
        return page;
    }
    }
}

QWidget* OptionsBar::buildCombinePage(ToolId id, bool withTolerance)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    // Quick Selection has no Intersect mode in CS6.
    addModeButtons(layout, page, id != ToolId::QuickSelection);

    if (withTolerance && controller_) {
        auto* spin = new NumericField(QStringLiteral("Tolerance"),
                                      numericConfig(0, 255, 1, 0, QString(), true,
                                                    QStringLiteral("optionsTolerance")),
                                      page);
        spin->setValue(controller_->tolerance());
        layout->addWidget(spin);
        connect(spin, &NumericField::valueChanged, this,
                [this](double value) { controller_->setTolerance(qRound(value)); });
    }

    if (id == ToolId::QuickSelection) {
        // ponytail: the brush pop-up and Refine Edge are deferred; Sample All
        // Layers and Auto-Enhance have no engine behind them, so they are shown
        // checked but disabled rather than as silent no-ops.
        auto* sample = new QCheckBox(QStringLiteral("Sample All Layers"), page);
        sample->setChecked(true);
        sample->setEnabled(false);
        sample->setToolTip(
            QStringLiteral("Not modelled: the wand samples the visible composite."));
        layout->addWidget(sample);
        auto* enhance = new QCheckBox(QStringLiteral("Auto-Enhance"), page);
        enhance->setChecked(true);
        enhance->setEnabled(false);
        enhance->setToolTip(
            QStringLiteral("Not modelled: Quick Selection has no edge-flow refinement."));
        layout->addWidget(enhance);
    }

    layout->addStretch(1);
    return page;
}

QWidget* OptionsBar::buildWandPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    addModeButtons(layout, page, true);

    auto* tolerance = new NumericField(QStringLiteral("Tolerance"),
                                       numericConfig(0, 255, 1, 0, QString(), true,
                                                     QStringLiteral("optionsTolerance")),
                                       page);
    tolerance->setValue(controller_ ? controller_->tolerance() : 32);
    layout->addWidget(tolerance);
    if (controller_) {
        connect(tolerance, &NumericField::valueChanged, this,
                [this](double value) { controller_->setTolerance(qRound(value)); });
    }

    auto* contiguous = new QCheckBox(QStringLiteral("Contiguous"), page);
    contiguous->setChecked(controller_ ? controller_->contiguous() : true);
    layout->addWidget(contiguous);
    if (controller_) {
        connect(contiguous, &QCheckBox::toggled, this,
                [this](bool on) { controller_->setContiguous(on); });
    }

    // ponytail: the engine rasteriser is binary (pixel-centre) and the wand
    // samples the visible composite; both controls stay visible but disabled.
    auto* antiAlias = new QCheckBox(QStringLiteral("Anti-alias"), page);
    antiAlias->setChecked(controller_ ? controller_->antiAlias() : true);
    antiAlias->setEnabled(false);
    antiAlias->setToolTip(
        QStringLiteral("Not modelled: the selection rasteriser is binary (pixel-centre)."));
    layout->addWidget(antiAlias);
    auto* sample = new QCheckBox(QStringLiteral("Sample All Layers"), page);
    sample->setChecked(controller_ ? controller_->sampleAllLayers() : true);
    sample->setEnabled(false);
    sample->setToolTip(QStringLiteral("Not modelled: the wand samples the visible composite."));
        layout->addWidget(sample);

    addRefineEdgeButton(layout, page);
    layout->addStretch(1);
    return page;
}

void OptionsBar::addModeButtons(QHBoxLayout* layout, QWidget* page, bool withIntersect)
{
    auto* group = new QButtonGroup(page);
    group->setExclusive(true);
    for (const ModeButton& modeButton : kModes) {
        if (!withIntersect && modeButton.mode == SelectionMode::Intersect) {
            continue;
        }
        auto* button = new QToolButton(page);
        button->setIcon(icon(QString::fromLatin1(modeButton.icon)));
        button->setIconSize(QSize(18, 18));
        button->setToolButtonStyle(Qt::ToolButtonIconOnly);
        button->setToolTip(QString::fromLatin1(modeButton.label));
        button->setCheckable(true);
        button->setChecked(controller_ && controller_->combineMode() == modeButton.mode);
        group->addButton(button);
        layout->addWidget(button);
        if (controller_) {
            connect(button, &QToolButton::clicked, this,
                    [this, modeButton]() { controller_->setCombineMode(modeButton.mode); });
        }
    }
}

QWidget* OptionsBar::buildSelectionPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    addModeButtons(layout, page, true);

    auto* feather = new NumericField(
        QStringLiteral("Feather"),
        numericConfig(0.0, 250.0, 1.0, 1, QStringLiteral("px"), false,
                      QStringLiteral("optionsFeather")),
        page);
    feather->setValue(controller_ ? controller_->feather() : 0.0);
    layout->addWidget(feather);
    if (controller_) {
        connect(feather, &NumericField::valueChanged, this,
                [this](double value) { controller_->setFeather(value); });
    }

    const bool isMarquee = id == ToolId::Marquee || id == ToolId::EllipticalMarquee;
    if (isMarquee) {
        layout->addWidget(new QLabel(QStringLiteral("Style"), page));
        auto* styleCombo = new QComboBox(page);
        styleCombo->addItem(QStringLiteral("Normal"), int(MarqueeStyle::Normal));
        styleCombo->addItem(QStringLiteral("Fixed Ratio"), int(MarqueeStyle::FixedRatio));
        styleCombo->addItem(QStringLiteral("Fixed Size"), int(MarqueeStyle::FixedSize));
        layout->addWidget(styleCombo);

        // ponytail: 1:1 and 100x100 are inferred defaults (CS6 Help does not
        // state shipped values); the in/cm unit combo is deferred, so size is px.
        auto* ratioBox = new QWidget(page);
        auto* ratioLayout = new QHBoxLayout(ratioBox);
        ratioLayout->setContentsMargins(0, 0, 0, 0);
        auto* ratioW = new NumericField(QString(),
                                        numericConfig(0.1, 100.0, 0.1, 1, QString(), true,
                                                      QStringLiteral("optionsRatioW")),
                                        ratioBox);
        ratioW->setValue(controller_ ? controller_->fixedRatioWidth() : 1.0);
        auto* ratioH = new NumericField(QString(),
                                        numericConfig(0.1, 100.0, 0.1, 1, QString(), true,
                                                      QStringLiteral("optionsRatioH")),
                                        ratioBox);
        ratioH->setValue(controller_ ? controller_->fixedRatioHeight() : 1.0);
        ratioLayout->addWidget(ratioW);
        ratioLayout->addWidget(new QLabel(QStringLiteral(":"), ratioBox));
        ratioLayout->addWidget(ratioH);
        ratioBox->setToolTip(QStringLiteral("Inferred default 1:1."));
        layout->addWidget(ratioBox);

        auto* sizeBox = new QWidget(page);
        auto* sizeLayout = new QHBoxLayout(sizeBox);
        sizeLayout->setContentsMargins(0, 0, 0, 0);
        auto* sizeW = new NumericField(QString(),
                                       numericConfig(1, 10000, 1, 0, QString(), true,
                                                     QStringLiteral("optionsSizeW")),
                                       sizeBox);
        sizeW->setValue(controller_ ? controller_->fixedSizeWidth() : 100);
        auto* sizeH = new NumericField(QString(),
                                       numericConfig(1, 10000, 1, 0, QString(), true,
                                                     QStringLiteral("optionsSizeH")),
                                       sizeBox);
        sizeH->setValue(controller_ ? controller_->fixedSizeHeight() : 100);
        sizeLayout->addWidget(sizeW);
        sizeLayout->addWidget(new QLabel(QStringLiteral("x"), sizeBox));
        sizeLayout->addWidget(sizeH);
        sizeBox->setToolTip(
            QStringLiteral("Inferred default 100x100 px; non-pixel units are deferred."));
        layout->addWidget(sizeBox);

        if (controller_) {
            connect(ratioW, &NumericField::valueChanged, this, [this, ratioH](double v) {
                controller_->setFixedRatio(v, ratioH->value());
            });
            connect(ratioH, &NumericField::valueChanged, this, [this, ratioW](double v) {
                controller_->setFixedRatio(ratioW->value(), v);
            });
            connect(sizeW, &NumericField::valueChanged, this, [this, sizeH](double v) {
                controller_->setFixedSize(qRound(v), qRound(sizeH->value()));
            });
            connect(sizeH, &NumericField::valueChanged, this, [this, sizeW](double v) {
                controller_->setFixedSize(qRound(sizeW->value()), qRound(v));
            });
        }

        const auto applyStyleVisibility = [ratioBox, sizeBox](int style) {
            ratioBox->setVisible(style == int(MarqueeStyle::FixedRatio));
            sizeBox->setVisible(style == int(MarqueeStyle::FixedSize));
        };
        const int initialStyle =
            controller_ ? int(controller_->marqueeStyle()) : int(MarqueeStyle::Normal);
        styleCombo->setCurrentIndex(styleCombo->findData(initialStyle));
        applyStyleVisibility(initialStyle);
        if (controller_) {
            connect(styleCombo, &QComboBox::currentIndexChanged, this,
                    [this, styleCombo, applyStyleVisibility](int) {
                        const int style = styleCombo->currentData().toInt();
                        controller_->setMarqueeStyle(static_cast<MarqueeStyle>(style));
                        applyStyleVisibility(style);
                    });
        }
    }

    if (id == ToolId::MagneticLasso) {
        addMagneticFields(layout, page);
    }

    if (id != ToolId::Marquee) {
        // ponytail: the engine rasterises at the pixel centre (binary coverage);
        // a fractional-coverage rasteriser is new engine work, so the control is
        // visible but disabled rather than a silent no-op.
        auto* antiAlias = new QCheckBox(QStringLiteral("Anti-alias"), page);
        antiAlias->setChecked(true);
        antiAlias->setEnabled(false);
        antiAlias->setToolTip(QStringLiteral(
            "Not modelled: the selection rasteriser is binary (pixel-centre)."));
        layout->addWidget(antiAlias);
    }

    addRefineEdgeButton(layout, page);
    layout->addStretch(1);
    return page;
}

void OptionsBar::addRefineEdgeButton(QHBoxLayout* layout, QWidget* page)
{
    auto* button = new QToolButton(page);
    button->setObjectName(QStringLiteral("optionsRefineEdge"));
    button->setText(QStringLiteral("Select and Mask…"));
    button->setToolButtonStyle(Qt::ToolButtonTextOnly);
    button->setAutoRaise(true);
    button->setFocusPolicy(Qt::NoFocus);
    button->setToolTip(QStringLiteral("Refine Edge (Ctrl+Alt+R)"));
    connect(button, &QToolButton::clicked, this, &OptionsBar::refineEdgeRequested);
    layout->addWidget(button);
}

void OptionsBar::addMagneticFields(QHBoxLayout* layout, QWidget* page)
{
    auto* width = new NumericField(QStringLiteral("Width"),
                                   numericConfig(1, 256, 1, 0, QStringLiteral("px"), true,
                                                 QStringLiteral("optionsMagneticWidth")),
                                   page);
    auto* contrast = new NumericField(QStringLiteral("Contrast"),
                                      numericConfig(1, 100, 1, 0, QStringLiteral("%"), true,
                                                    QStringLiteral("optionsMagneticContrast")),
                                      page);
    auto* frequency = new NumericField(QStringLiteral("Frequency"),
                                       numericConfig(0, 100, 1, 0, QString(), true,
                                                     QStringLiteral("optionsMagneticFrequency")),
                                       page);
    layout->addWidget(width);
    layout->addWidget(contrast);
    layout->addWidget(frequency);
    if (controller_) {
        width->setValue(controller_->magneticWidth());
        contrast->setValue(controller_->magneticContrast());
        frequency->setValue(controller_->magneticFrequency());
        connect(width, &NumericField::valueChanged, this,
                [this](double v) { controller_->setMagneticWidth(qRound(v)); });
        connect(contrast, &NumericField::valueChanged, this,
                [this](double v) { controller_->setMagneticContrast(qRound(v)); });
        connect(frequency, &NumericField::valueChanged, this,
                [this](double v) { controller_->setMagneticFrequency(qRound(v)); });
        // `[` / `]` change the width from the keyboard.
        connect(controller_, &ToolController::magneticWidthChanged, width,
                [width](int v) { width->setValue(v); });
    }
    // ponytail: pen pressure is not read, so Stylus Pressure is shown off and
    // disabled rather than as a silent no-op.
    auto* pressure = new QCheckBox(QStringLiteral("Stylus Pressure"), page);
    pressure->setEnabled(false);
    pressure->setToolTip(QStringLiteral("Not modelled: tablet pressure is not read."));
    layout->addWidget(pressure);
}

// The annotation tools' bars, as photorust ports them: the Ruler's X, Y, W, H,
// A, D1 readout (pixels and degrees), then Clear for every tool.
// ponytail: no Sample Size for samplers, Straighten for the Ruler, or Author
// and Color for notes yet.
QWidget* OptionsBar::buildAnnotationPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    if (id == ToolId::Ruler) {
        auto* readout = new QLabel(page);
        readout->setObjectName(QStringLiteral("optionsRulerReadout"));
        readout->setTextInteractionFlags(Qt::TextSelectableByMouse);
        layout->addWidget(readout);
        const auto update = [this, readout]() {
            PictureView* v = controller_ ? controller_->view() : nullptr;
            const ::rust::Vec<double> m = v ? ruler_measurement(*v) : ::rust::Vec<double>();
            QStringList fields;
            const char* names[] = {"X", "Y", "W", "H", "A", "D1"};
            for (int i = 0; i < 6; ++i) {
                const QString value = m.size() == 6
                    ? QString::number(m[i], 'f', 1) + (i == 4 ? QStringLiteral("°") : QString())
                    : QString();
                fields << QStringLiteral("%1: %2").arg(QLatin1String(names[i]), value);
            }
            readout->setText(fields.join(QStringLiteral("   ")));
        };
        update();
        if (controller_) {
            connect(controller_, &ToolController::rulerChanged, readout, update);
        }
    }

    auto* clear = new QToolButton(page);
    clear->setObjectName(QStringLiteral("optionsAnnotationClear"));
    clear->setText(id == ToolId::Note ? QStringLiteral("Clear All") : QStringLiteral("Clear"));
    clear->setToolTip(id == ToolId::ColorSampler ? QStringLiteral("Delete every color sampler")
                      : id == ToolId::Note       ? QStringLiteral("Delete every note")
                                                 : QStringLiteral("Remove the measuring line"));
    layout->addWidget(clear);
    if (controller_) {
        connect(clear, &QToolButton::clicked, this, [this]() { controller_->clearAnnotations(); });
    }
    layout->addStretch(1);
    return page;
}

QWidget* OptionsBar::buildPaintPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    addBrushTipFields(layout, page);
    auto addField = [&](const QString& label, const QString& name, const QString& suffix, int lo,
                        int hi, int value, void (ToolController::*setter)(int)) {
        auto* field =
            new NumericField(label, numericConfig(lo, hi, 1, 0, suffix, true, name), page);
        field->setValue(value);
        layout->addWidget(field);
        if (controller_) {
            connect(field, &NumericField::valueChanged, this,
                    [this, setter](double v) { (controller_->*setter)(qRound(v)); });
        }
    };

    addField(QStringLiteral("Opacity"), QStringLiteral("optionsBrushOpacity"),
             QStringLiteral("%"), 0, 100, controller_ ? controller_->brushOpacity() : 100,
             &ToolController::setBrushOpacity);
    addField(QStringLiteral("Flow"), QStringLiteral("optionsBrushFlow"), QStringLiteral("%"), 0,
             100, controller_ ? controller_->brushFlow() : 100, &ToolController::setBrushFlow);

    layout->addWidget(new QLabel(QStringLiteral("Mode"), page));
    auto* combo = new QComboBox(page);
    combo->addItem(QStringLiteral("Normal"), QStringLiteral("normal"));
    combo->addItem(QStringLiteral("Dissolve"), QStringLiteral("dissolve"));
    combo->addItem(QStringLiteral("Behind"), QStringLiteral("behind"));
    combo->addItem(QStringLiteral("Clear"), QStringLiteral("clear"));
    if (controller_) {
        combo->setCurrentIndex(combo->findData(controller_->brushMode()));
        connect(combo, &QComboBox::currentIndexChanged, this, [this, combo](int) {
            controller_->setBrushMode(combo->currentData().toString());
        });
    }
    layout->addWidget(combo);

    if (id == ToolId::Pencil) {
        auto* check = new QCheckBox(QStringLiteral("Auto Erase"), page);
        if (controller_) {
            check->setChecked(controller_->autoErase());
            connect(check, &QCheckBox::toggled, this,
                    [this](bool on) { controller_->setAutoErase(on); });
        }
        layout->addWidget(check);
    }

    layout->addStretch(1);
    return page;
}

// The healing tools' bars. Spot Healing picks its Type; Healing Brush toggles
// Aligned. The brush tip is shared with the paint tools through the same
// controller fields.
// ponytail: no Mode, Source (Pattern), or Sample (All Layers) controls yet.
QWidget* OptionsBar::buildHealingPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    addBrushTipFields(layout, page);

    if (id == ToolId::SpotHealingBrush) {
        layout->addWidget(new QLabel(QStringLiteral("Type"), page));
        auto* combo = new QComboBox(page);
        combo->setObjectName(QStringLiteral("optionsSpotHealingType"));
        combo->addItem(QStringLiteral("Proximity Match"));
        combo->addItem(QStringLiteral("Create Texture"));
        combo->addItem(QStringLiteral("Content-Aware"));
        if (controller_) {
            combo->setCurrentIndex(controller_->spotHealingType());
            connect(combo, &QComboBox::currentIndexChanged, this,
                    [this](int index) { controller_->setSpotHealingType(index); });
        }
        layout->addWidget(combo);
    } else {
        auto* check = new QCheckBox(QStringLiteral("Aligned"), page);
        check->setObjectName(QStringLiteral("optionsHealingAligned"));
        if (controller_) {
            check->setChecked(controller_->healingAligned());
            connect(check, &QCheckBox::toggled, this,
                    [this](bool on) { controller_->setHealingAligned(on); });
        }
        layout->addWidget(check);
    }

    layout->addStretch(1);
    return page;
}

// The Count (Extended) options bar: the running total, the count-group dropdown
// with its eye / new / delete controls, Clear, the group colour swatch, and the
// marker and label sizes.
QWidget* OptionsBar::buildCountPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    auto* total = new QLabel(page);
    total->setObjectName(QStringLiteral("optionsCountTotal"));
    layout->addWidget(total);

    auto* group = new QComboBox(page);
    group->setObjectName(QStringLiteral("optionsCountGroup"));
    group->setMinimumWidth(130);
    layout->addWidget(group);

    auto* eye = new QToolButton(page);
    eye->setObjectName(QStringLiteral("optionsCountVisible"));
    eye->setCheckable(true);
    eye->setToolTip(QStringLiteral("Toggle count group visibility"));
    layout->addWidget(eye);

    auto* addGroup = new QToolButton(page);
    addGroup->setObjectName(QStringLiteral("optionsCountAddGroup"));
    addGroup->setIcon(icon(QStringLiteral("count.newGroup")));
    addGroup->setToolTip(QStringLiteral("Create a count group"));
    layout->addWidget(addGroup);

    auto* delGroup = new QToolButton(page);
    delGroup->setObjectName(QStringLiteral("optionsCountDeleteGroup"));
    delGroup->setIcon(icon(QStringLiteral("layers.delete")));
    delGroup->setToolTip(QStringLiteral("Delete the count group"));
    layout->addWidget(delGroup);

    auto* clear = new QToolButton(page);
    clear->setObjectName(QStringLiteral("optionsCountClear"));
    clear->setText(QStringLiteral("Clear"));
    clear->setToolTip(QStringLiteral("Reset the active group's count to 0"));
    layout->addWidget(clear);

    auto* color = new QToolButton(page);
    color->setObjectName(QStringLiteral("optionsCountColor"));
    color->setToolTip(QStringLiteral("Count group colour"));
    layout->addWidget(color);

    auto* marker = new NumericField(
        QStringLiteral("Marker Size"),
        numericConfig(1, 10, 1, 0, QString(), true, QStringLiteral("optionsCountMarkerSize")),
        page);
    layout->addWidget(marker);
    auto* label = new NumericField(
        QStringLiteral("Label Size"),
        numericConfig(8, 72, 1, 0, QString(), true, QStringLiteral("optionsCountLabelSize")),
        page);
    layout->addWidget(label);

    if (controller_) {
        const auto refresh = [this, total, group, eye, addGroup, delGroup, clear, color, marker,
                              label]() {
            PictureView* v = controller_->view();
            const bool have = v && v->has_document();
            const int groups = have ? count_group_count(*v) : 0;
            const int active = have ? count_active_group(*v) : 0;
            {
                const QSignalBlocker block(group);
                group->clear();
                for (int g = 0; g < groups; ++g) {
                    group->addItem(count_group_name(*v, g));
                }
                if (active >= 0 && active < group->count()) {
                    group->setCurrentIndex(active);
                }
            }
            const bool hasGroup = have && active >= 0 && active < groups;
            total->setText(hasGroup
                               ? QStringLiteral("Count: %1").arg(count_group_total(*v, active))
                               : QStringLiteral("Count:"));
            const bool visible = hasGroup && count_group_visible(*v, active);
            eye->setChecked(visible);
            eye->setIcon(icon(visible ? QStringLiteral("layers.eyeOn")
                                      : QStringLiteral("layers.eyeOff")));
            delGroup->setEnabled(groups > 1);
            clear->setEnabled(hasGroup);
            marker->setEnabled(hasGroup);
            label->setEnabled(hasGroup);
            color->setEnabled(hasGroup);
            if (hasGroup) {
                QPixmap swatch(18, 18);
                swatch.fill(QColor(QRgb(count_group_color(*v, active))));
                color->setIcon(QIcon(swatch));
                const QSignalBlocker b1(marker);
                const QSignalBlocker b2(label);
                marker->setValue(count_group_marker_size(*v, active));
                label->setValue(count_group_label_size(*v, active));
            }
        };

        connect(group, &QComboBox::currentIndexChanged, this, [this, refresh](int index) {
            if (PictureView* v = controller_->view(); v && index >= 0) {
                count_set_active_group(*v, index);
            }
            refresh();
        });
        connect(eye, &QToolButton::toggled, this, [this, refresh](bool on) {
            if (PictureView* v = controller_->view()) {
                count_set_visible(*v, count_active_group(*v), on);
            }
            refresh();
        });
        connect(addGroup, &QToolButton::clicked, this, [this, refresh]() {
            PictureView* v = controller_->view();
            if (!v) {
                return;
            }
            const int n = count_group_count(*v) + 1;
            bool ok = false;
            const QString name = QInputDialog::getText(
                this, QStringLiteral("Count Group Name"), QStringLiteral("Count Group Name:"),
                QLineEdit::Normal, QStringLiteral("Count Group %1").arg(n), &ok);
            if (ok && !name.isEmpty()) {
                count_add_group(*v, name);
            }
            refresh();
        });
        connect(delGroup, &QToolButton::clicked, this, [this, refresh]() {
            if (PictureView* v = controller_->view()) {
                count_remove_group(*v, count_active_group(*v));
            }
            refresh();
        });
        connect(clear, &QToolButton::clicked, this, [this, refresh]() {
            if (PictureView* v = controller_->view()) {
                count_clear(*v);
            }
            refresh();
        });
        connect(color, &QToolButton::clicked, this, [this, refresh]() {
            PictureView* v = controller_->view();
            if (!v) {
                return;
            }
            const int active = count_active_group(*v);
            const QColor chosen = ColorPickerDialog::getColor(
                QColor(QRgb(count_group_color(*v, active))), this,
                QStringLiteral("Count Group Color"));
            if (chosen.isValid()) {
                count_set_color(*v, active, chosen.rgb() & 0xffffff);
            }
            refresh();
        });
        connect(marker, &NumericField::valueChanged, this, [this, refresh](double value) {
            if (PictureView* v = controller_->view()) {
                count_set_marker_size(*v, count_active_group(*v), qRound(value));
            }
            refresh();
        });
        connect(label, &NumericField::valueChanged, this, [this, refresh](double value) {
            if (PictureView* v = controller_->view()) {
                count_set_label_size(*v, count_active_group(*v), qRound(value));
            }
            refresh();
        });
        connect(controller_, &ToolController::countChanged, this, [refresh]() { refresh(); });
        refresh();
    }

    layout->addStretch(1);
    return page;
}

// CS6's Patch bar, left to right: the selection combine buttons, the Patch
// mode, the Source/Destination pair, Transparent, and Use Pattern. Source,
// Destination, and Transparent describe sampling from the drag, which
// Content-Aware does not do, so they disable together with it.
// ponytail: no Adaptation or Sample All Layers; Use Pattern is shown disabled.
QWidget* OptionsBar::buildPatchPage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addModeButtons(layout, page, true);

    layout->addWidget(new QLabel(QStringLiteral("Patch:"), page));
    auto* mode = new QComboBox(page);
    mode->setObjectName(QStringLiteral("optionsPatchMode"));
    mode->addItem(QStringLiteral("Normal"));
    mode->addItem(QStringLiteral("Content-Aware"));
    layout->addWidget(mode);

    auto* direction = new QButtonGroup(page);
    direction->setExclusive(true);
    auto* source = new QToolButton(page);
    source->setObjectName(QStringLiteral("optionsPatchSource"));
    source->setText(QStringLiteral("Source"));
    source->setToolTip(QStringLiteral("The selection is the flaw; drag it onto the pixels to "
                                      "repair it with"));
    auto* destination = new QToolButton(page);
    destination->setObjectName(QStringLiteral("optionsPatchDestination"));
    destination->setText(QStringLiteral("Destination"));
    destination->setToolTip(QStringLiteral("The selection is good material; drag it onto the "
                                           "area to repair"));
    for (QToolButton* button : {source, destination}) {
        button->setCheckable(true);
        button->setAutoRaise(true);
        direction->addButton(button);
        layout->addWidget(button);
    }

    auto* transparent = new QCheckBox(QStringLiteral("Transparent"), page);
    transparent->setObjectName(QStringLiteral("optionsPatchTransparent"));
    transparent->setToolTip(QStringLiteral("Transfer only the source's texture, keeping the "
                                           "patched area's own colour"));
    layout->addWidget(transparent);

    auto* usePattern = new QToolButton(page);
    usePattern->setText(QStringLiteral("Use Pattern"));
    usePattern->setToolTip(QStringLiteral("Use Pattern: not implemented yet"));
    usePattern->setEnabled(false);
    layout->addWidget(usePattern);

    const auto syncEnabled = [source, destination, transparent](bool contentAware) {
        source->setEnabled(!contentAware);
        destination->setEnabled(!contentAware);
        transparent->setEnabled(!contentAware);
    };
    if (controller_) {
        mode->setCurrentIndex(controller_->patchContentAware() ? 1 : 0);
        (controller_->patchDestination() ? destination : source)->setChecked(true);
        transparent->setChecked(controller_->patchTransparent());
        connect(mode, &QComboBox::currentIndexChanged, this, [this, syncEnabled](int index) {
            controller_->setPatchContentAware(index == 1);
            syncEnabled(index == 1);
        });
        connect(destination, &QToolButton::toggled, this,
                [this](bool on) { controller_->setPatchDestination(on); });
        connect(transparent, &QCheckBox::toggled, this,
                [this](bool on) { controller_->setPatchTransparent(on); });
    } else {
        source->setChecked(true);
    }
    syncEnabled(mode->currentIndex() == 1);

    layout->addStretch(1);
    return page;
}

// CS6's Content-Aware Move bar: the selection combine buttons, Mode (Move /
// Extend), Adaptation (Very Strict … Very Loose, default Medium), and Sample
// All Layers.
// ponytail: Sample All Layers is shown disabled; the active layer is sampled.
QWidget* OptionsBar::buildContentAwareMovePage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));
    addModeButtons(layout, page, true);

    layout->addWidget(new QLabel(QStringLiteral("Mode:"), page));
    auto* mode = new QComboBox(page);
    mode->setObjectName(QStringLiteral("optionsContentAwareMoveMode"));
    mode->addItem(QStringLiteral("Move"));
    mode->addItem(QStringLiteral("Extend"));
    layout->addWidget(mode);

    layout->addWidget(new QLabel(QStringLiteral("Adaptation:"), page));
    auto* adaptation = new QComboBox(page);
    adaptation->setObjectName(QStringLiteral("optionsContentAwareAdaptation"));
    for (const char* level : {"Very Strict", "Strict", "Medium", "Loose", "Very Loose"}) {
        adaptation->addItem(QString::fromLatin1(level));
    }
    adaptation->setToolTip(QStringLiteral("How closely the fill reflects the existing image "
                                          "patterns"));
    layout->addWidget(adaptation);

    auto* sampleAll = new QCheckBox(QStringLiteral("Sample All Layers"), page);
    sampleAll->setToolTip(QStringLiteral("Sample All Layers: not implemented yet"));
    sampleAll->setEnabled(false);
    layout->addWidget(sampleAll);

    if (controller_) {
        mode->setCurrentIndex(controller_->contentAwareMoveExtend() ? 1 : 0);
        adaptation->setCurrentIndex(controller_->contentAwareAdaptation());
        connect(mode, &QComboBox::currentIndexChanged, this,
                [this](int index) { controller_->setContentAwareMoveExtend(index == 1); });
        connect(adaptation, &QComboBox::currentIndexChanged, this,
                [this](int index) { controller_->setContentAwareAdaptation(index); });
    } else {
        adaptation->setCurrentIndex(2);
    }

    layout->addStretch(1);
    return page;
}

QToolButton* OptionsBar::toolButton(ToolId id, QWidget* parent)
{
    auto* button = new QToolButton(parent);
    button->setIcon(icon(QStringLiteral("tool.") + toolIdName(id)));
    button->setIconSize(QSize(18, 18));
    button->setToolButtonStyle(Qt::ToolButtonIconOnly);
    button->setToolTip(QString::fromLatin1(toolInfo(id).label));
    auto* menu = new QMenu(button);
    QAction* presets = menu->addAction(QStringLiteral("Presets (coming soon)"));
    presets->setEnabled(false);
    button->setMenu(menu);
    button->setPopupMode(QToolButton::InstantPopup);
    return button;
}

void OptionsBar::showTool(ToolId id)
{
    if (!stack_) {
        return;
    }
    stack_->setCurrentIndex(static_cast<int>(id));
    // A stacked widget reserves the widest page's minimum; ignore the hidden
    // pages so only the active tool's options set the bar's (and window's)
    // minimum width.
    for (int i = 0; i < stack_->count(); ++i) {
        stack_->widget(i)->setSizePolicy(i == stack_->currentIndex() ? QSizePolicy::Preferred
                                                                     : QSizePolicy::Ignored,
                                         QSizePolicy::Preferred);
    }
    stack_->updateGeometry();
    // Re-seed the Crop W/H fields for their mode (ratio values or pixels).
    if (id == ToolId::Crop) {
        syncCropFields();
    }
}

QWidget* OptionsBar::pageForTest(ToolId id) const
{
    return stack_ ? stack_->widget(static_cast<int>(id)) : nullptr;
}

} // namespace pictura
