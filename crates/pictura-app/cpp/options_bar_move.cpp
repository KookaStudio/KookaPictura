// The Move tool's options bar: CS6's six Align and six Distribute buttons.
// Part of OptionsBar; ported from photorust's MainWindow::addMoveOptions.

#include "options_bar.h"

#include "icons.h"

#include <QtGui/QAction>
#include <QtGui/QActionGroup>
#include <QtWidgets/QCheckBox>
#include <QtWidgets/QComboBox>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QMenu>
#include <QtWidgets/QStackedWidget>
#include <QtWidgets/QToolButton>

namespace pictura {

namespace {

// Menu order, which is also the engine's edge index (0 Top ... 5 Right).
constexpr const char* kEdges[] = {"Top",  "VerticalCenter",   "Bottom",
                                  "Left", "HorizontalCenter", "Right"};
constexpr const char* kEdgeTips[] = {"top edges",  "vertical centers",   "bottom edges",
                                     "left edges", "horizontal centers", "right edges"};

} // namespace

// The Move tool's options bar: Auto-Select, Show Transform Controls, CS6's six
// Align and six Distribute buttons, and a three-dots menu with the same actions
// plus the Align To: Selection/Canvas choice.
QWidget* OptionsBar::buildMovePage(ToolId id)
{
    auto* page = new QWidget(stack_);
    auto* layout = new QHBoxLayout(page);
    layout->setContentsMargins(4, 2, 4, 2);
    layout->addWidget(toolButton(id, page));

    auto* autoSelect = new QComboBox(page);
    autoSelect->setObjectName(QStringLiteral("optionsMoveAutoSelect"));
    autoSelect->addItem(QStringLiteral("Group"), 0);
    autoSelect->addItem(QStringLiteral("Layer"), 1);
    autoSelect->setCurrentIndex(controller_ ? controller_->moveAutoSelect() : 0);
    autoSelect->setToolTip(QStringLiteral("Auto-Select"));
    layout->addWidget(autoSelect);
    // ponytail: Auto-Select is recorded but not wired; a deep hit test for the
    // topmost layer under the pointer is new engine work, so the move stays on
    // the Layers panel's active layer.
    auto* transform = new QCheckBox(QStringLiteral("Show Transform Controls"), page);
    transform->setObjectName(QStringLiteral("optionsMoveTransform"));
    transform->setChecked(controller_ && controller_->moveShowTransformControls());
    layout->addWidget(transform);
    // ponytail: Show Transform Controls is recorded but does not yet show the
    // Free Transform box on the Move tool; Ctrl+T still starts it.

    // The bar aligns against the selection when there is one (CS6's Align
    // Layers To Selection), else the selected layers against each other.
    const auto addRow = [&](bool distribute) {
        for (int edge = 0; edge < 6; ++edge) {
            const QString name = QLatin1String(distribute ? "distribute" : "align")
                + QLatin1String(kEdges[edge]);
            auto* button = new QToolButton(page);
            button->setObjectName(QStringLiteral("options") + name.at(0).toUpper() + name.mid(1));
            button->setIcon(icon(QStringLiteral("layer.") + name));
            button->setIconSize(QSize(16, 16));
            button->setAutoRaise(true);
            button->setEnabled(false);
            button->setToolTip(QLatin1String(distribute ? "Distribute " : "Align ")
                               + QLatin1String(kEdgeTips[edge]));
            connect(button, &QToolButton::clicked, this, [this, distribute, edge]() {
                if (distribute) {
                    emit distributeRequested(edge);
                } else {
                    emit alignRequested(edge);
                }
            });
            (distribute ? distributeButtons_ : alignButtons_).push_back(button);
            layout->addWidget(button);
            // A gap between the vertical three and the horizontal three.
            if (edge == 2) {
                layout->addSpacing(3);
            }
        }
    };
    addRow(false);
    layout->addSpacing(6);
    addRow(true);

    auto* menuButton = new QToolButton(page);
    menuButton->setObjectName(QStringLiteral("optionsMoveMenu"));
    menuButton->setText(QString::fromUtf8("\xE2\x80\xA6"));
    menuButton->setToolTip(QStringLiteral("Align and distribute options"));
    menuButton->setPopupMode(QToolButton::InstantPopup);
    auto* menu = new QMenu(menuButton);
    for (int edge = 0; edge < 6; ++edge) {
        QAction* action = menu->addAction(QLatin1String("Align ") + QLatin1String(kEdgeTips[edge]));
        action->setObjectName(QStringLiteral("moveMenuAlign%1").arg(QLatin1String(kEdges[edge])));
        connect(action, &QAction::triggered, this, [this, edge]() { emit alignRequested(edge); });
    }
    menu->addSeparator();
    for (int edge = 0; edge < 6; ++edge) {
        QAction* action =
            menu->addAction(QLatin1String("Distribute ") + QLatin1String(kEdgeTips[edge]));
        action->setObjectName(
            QStringLiteral("moveMenuDistribute%1").arg(QLatin1String(kEdges[edge])));
        connect(action, &QAction::triggered, this, [this, edge]() { emit distributeRequested(edge); });
    }
    menu->addSeparator();
    QAction* header = menu->addAction(QStringLiteral("Align To:"));
    header->setEnabled(false);
    auto* alignToGroup = new QActionGroup(menu);
    alignToGroup->setExclusive(true);
    QAction* toSelectionAction = menu->addAction(QStringLiteral("Selection"));
    toSelectionAction->setObjectName(QStringLiteral("moveMenuAlignToSelection"));
    QAction* toCanvasAction = menu->addAction(QStringLiteral("Canvas"));
    toCanvasAction->setObjectName(QStringLiteral("moveMenuAlignToCanvas"));
    for (QAction* action : {toSelectionAction, toCanvasAction}) {
        action->setCheckable(true);
        alignToGroup->addAction(action);
    }
    const int alignTo = controller_ ? controller_->moveAlignTo() : 0;
    toSelectionAction->setChecked(alignTo == 0);
    toCanvasAction->setChecked(alignTo == 1);
    menuButton->setMenu(menu);
    layout->addWidget(menuButton);

    if (controller_) {
        connect(autoSelect, &QComboBox::currentIndexChanged, this,
                [this, autoSelect](int) { controller_->setMoveAutoSelect(autoSelect->currentIndex()); });
        connect(transform, &QCheckBox::toggled, this,
                [this](bool on) { controller_->setMoveShowTransformControls(on); });
        connect(toSelectionAction, &QAction::triggered, this,
                [this]() { controller_->setMoveAlignTo(0); });
        connect(toCanvasAction, &QAction::triggered, this,
                [this]() { controller_->setMoveAlignTo(1); });
    }

    layout->addStretch(1);
    return page;
}

void OptionsBar::setAlignEnabled(bool align, bool distribute)
{
    for (QToolButton* button : alignButtons_) {
        button->setEnabled(align);
    }
    for (QToolButton* button : distributeButtons_) {
        button->setEnabled(distribute);
    }
}

} // namespace pictura
