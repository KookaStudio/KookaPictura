#include "numeric_field.h"

#include "icons.h"
#include "jump_slider.h"

#include <QtCore/QEvent>
#include <QtCore/QLocale>
#include <QtCore/QSize>
#include <QtGui/QDoubleValidator>
#include <QtGui/QIntValidator>
#include <QtGui/QKeyEvent>
#include <QtGui/QMouseEvent>
#include <QtWidgets/QHBoxLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QLineEdit>
#include <QtWidgets/QSlider>
#include <QtWidgets/QStyleOptionFrame>
#include <QtWidgets/QToolButton>
#include <QtWidgets/QVBoxLayout>

#include <algorithm>
#include <cmath>

namespace pictura {

namespace {

// One physical pixel is one step; Shift coarsens, Ctrl refines.
double modifierScale(Qt::KeyboardModifiers modifiers)
{
    if (modifiers.testFlag(Qt::ShiftModifier)) {
        return 10.0;
    }
    if (modifiers.testFlag(Qt::ControlModifier)) {
        return 0.1;
    }
    return 1.0;
}

// QLineEdit insets its text by a fixed 2px inside the style's content rect.
constexpr int kTextInset = 2;

// Where the style lets `edit` draw its text, inside the frame and padding.
QRect lineEditContents(const QLineEdit* edit)
{
    QStyleOptionFrame option;
    option.initFrom(edit);
    option.rect = edit->rect();
    option.lineWidth = edit->hasFrame()
        ? edit->style()->pixelMetric(QStyle::PM_DefaultFrameWidth, &option, edit)
        : 0;
    option.state |= QStyle::State_Sunken;
    return edit->style()->subElementRect(QStyle::SE_LineEditContents, &option, edit);
}

// The edit's frame, inside its border but outside its padding: the inset
// widgets (the suffix and the popup arrow) hug this rect.
QRect lineEditFrame(const QLineEdit* edit)
{
    QStyleOptionFrame option;
    option.initFrom(edit);
    option.rect = edit->rect();
    const int border = edit->hasFrame()
        ? edit->style()->pixelMetric(QStyle::PM_DefaultFrameWidth, &option, edit)
        : 0;
    return edit->rect().adjusted(border, border, -border, -border);
}

} // namespace

NumericField::NumericField(const QString& label, const NumericFieldConfig& config,
                          QWidget* parent)
    : QWidget(parent)
    , config_(config)
    , value_(config.minimum)
{
    if (!config_.objectName.isEmpty()) {
        setObjectName(config_.objectName);
    }

    auto* layout = new QHBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(3);

    if (!label.isEmpty()) {
        label_ = new QLabel(label, this);
        label_->setObjectName(config_.namePrefix + QStringLiteral("Label"));
        label_->setCursor(Qt::SizeHorCursor);
        label_->installEventFilter(this);
        layout->addWidget(label_);
    }

    edit_ = new QLineEdit(this);
    edit_->setObjectName(config_.namePrefix + QStringLiteral("Edit"));
    edit_->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    // A fractional-capable integer field (the crop W/H ratio) takes up to six
    // decimals and keeps a typed fraction; a plain integer field rejects one.
    // The C locale keeps '.' the decimal separator regardless of the user's
    // locale, matching QString::toDouble and the values the fields emit.
    const int validatorDecimals = config_.allowFractional && config_.decimals == 0
        ? 6
        : config_.decimals;
    auto* validator = new QDoubleValidator(config_.minimum, config_.maximum, validatorDecimals,
                                           edit_);
    validator->setLocale(QLocale::c());
    validator->setNotation(QDoubleValidator::StandardNotation);
    edit_->setValidator(validator);
    edit_->setText(formatValue(value_));
    edit_->installEventFilter(this);
    layout->addWidget(edit_);

    if (!config_.suffix.isEmpty()) {
        suffix_ = new QLabel(config_.suffix, edit_);
        suffix_->setObjectName(config_.namePrefix + QStringLiteral("Suffix"));
        suffix_->setCursor(Qt::SizeHorCursor);
        suffix_->installEventFilter(this);
    }

    reserveWidth();

    if (config_.popup) {
        // The arrow lives inside the edit, flush to its right edge, like the
        // suffix; the QSS `QLineEdit QToolButton` rule draws the hairline
        // separator. It is checkable so it stays darker while the popup is open.
        arrow_ = new QToolButton(edit_);
        arrow_->setObjectName(config_.namePrefix + QStringLiteral("Arrow"));
        arrow_->setAutoRaise(true);
        arrow_->setCheckable(true);
        arrow_->setFocusPolicy(Qt::NoFocus);
        // The arrow is a child of the text editor, which shows an I-beam; use
        // the default arrow cursor so the button reads as a button.
        arrow_->setCursor(Qt::ArrowCursor);
        arrow_->setArrowType(Qt::NoArrow);
        arrow_->setIcon(icon(QStringLiteral("chevron.down")));
        arrow_->setIconSize(QSize(12, 12));
        // 18 px matches the select's drop-down cell, leaving the 12 px icon room
        // after the 2 px padding and 1 px hairline.
        arrow_->setFixedWidth(18);
        arrow_->setToolTip(tr("Slider"));

        popup_ = new QWidget(this, Qt::Popup);
        popup_->setObjectName(config_.namePrefix + QStringLiteral("Popup"));
        auto* popupLayout = new QVBoxLayout(popup_);
        popupLayout->setContentsMargins(6, 6, 6, 6);
        slider_ = new JumpSlider(Qt::Horizontal, popup_);
        slider_->setRange(int(config_.minimum), int(config_.maximum));
        slider_->setPageStep(int(config_.page));
        slider_->setValue(int(std::lround(value_)));
        slider_->setMinimumWidth(120);
        popupLayout->addWidget(slider_);

        connect(slider_, &QSlider::valueChanged, this, [this](int v) {
            if (!syncing_) {
                applyUserValue(v);
            }
        });
        connect(slider_, &QSlider::sliderReleased, this, [this] { commitPending(); });
        connect(arrow_, &QToolButton::clicked, this, [this] { showPopup(); });
        popup_->installEventFilter(this);
        reserveWidth();
    }

    connect(edit_, &QLineEdit::editingFinished, this, [this] { commitEdit(); });
    connect(edit_, &QLineEdit::textChanged, this, [this] { layoutInset(); });
}

void NumericField::reserveWidth()
{
    // Measured against the polished style so the widest value, the suffix, and
    // the inset arrow fit without QLineEdit scrolling the start of the text out
    // of view.
    edit_->ensurePolished();
    const QFontMetrics metrics = edit_->fontMetrics();
    const int valueWidth = std::max(metrics.horizontalAdvance(formatValue(config_.maximum)),
                                    metrics.horizontalAdvance(formatValue(config_.minimum)));
    const int suffixWidth =
        suffix_ && !suffix_->isHidden() ? metrics.horizontalAdvance(config_.suffix) : 0;
    const int arrowWidth = arrow_ ? arrow_->width() : 0;
    const int frame = edit_->width() - lineEditContents(edit_).width();
    // One extra pixel each for the text cursor and rounding.
    edit_->setFixedWidth(frame + 2 * kTextInset + valueWidth + suffixWidth + arrowWidth + 2);
    layoutInset();
}

QString NumericField::labelText() const
{
    return label_ ? label_->text() : QString();
}

void NumericField::setLabelAlignment(Qt::Alignment alignment)
{
    if (label_) {
        label_->setAlignment(alignment);
    }
}

QString NumericField::formatValue(double value) const
{
    if (config_.decimals == 0) {
        // An integer field shows an integer unless it holds a real fraction
        // (a typed decimal), which is kept.
        if (std::isfinite(value) && value == std::floor(value)) {
            return QString::number(qint64(std::llround(value)));
        }
        return QString::number(value, 'g', 6);
    }
    return QString::number(value, 'f', config_.decimals);
}

void NumericField::setValue(double value)
{
    decimalTyped_ = false;
    blank_ = false;
    value_ = std::clamp(value, config_.minimum, config_.maximum);
    edit_->setText(formatValue(value_));
    syncSlider();
}

void NumericField::setBlank(bool blank)
{
    blank_ = blank;
    if (blank) {
        edit_->clear();
    } else {
        edit_->setText(formatValue(value_));
    }
}

void NumericField::setSuffixVisible(bool visible)
{
    if (!suffix_) {
        return;
    }
    suffix_->setVisible(visible);
    reserveWidth();
}

void NumericField::syncSlider()
{
    if (!slider_) {
        return;
    }
    syncing_ = true;
    slider_->setValue(int(std::lround(value_)));
    syncing_ = false;
}

void NumericField::commitEdit()
{
    if (edit_->text().trimmed().isEmpty()) {
        if (blank_) {
            return;
        }
        edit_->setText(formatValue(value_));
        return;
    }
    blank_ = false;
    bool ok = false;
    const double entered = edit_->text().toDouble(&ok);
    if (!ok) {
        edit_->setText(formatValue(value_));
        return;
    }
    decimalTyped_ = edit_->text().contains(QLatin1Char('.'));
    applyUserValue(entered);
    commitPending();
}

void NumericField::applyUserValue(double value)
{
    blank_ = false;
    value = std::clamp(value, config_.minimum, config_.maximum);
    if (config_.decimals == 0 && !decimalTyped_) {
        value = std::round(value);
    }
    if (value == value_) {
        edit_->setText(formatValue(value_));
        return;
    }
    value_ = value;
    edit_->setText(formatValue(value_));
    syncSlider();
    emit valueChanged(value_);
    pending_ = true;
}

void NumericField::commitPending()
{
    if (pending_) {
        pending_ = false;
        emit valueCommitted(value_);
    }
}

void NumericField::layoutInset()
{
    if (!edit_) {
        return;
    }
    const int arrowWidth = arrow_ ? arrow_->width() : 0;
    int suffixWidth = 0;
    if (suffix_) {
        suffix_->adjustSize();
        // `isHidden`, not `isVisible`: the field may not be shown yet.
        suffixWidth = suffix_->isHidden() ? 0 : suffix_->width();
    }
    // The right margin keeps typed text from running under the suffix or the
    // arrow, both of which are children of the edit.
    edit_->setTextMargins(0, 0, suffixWidth + arrowWidth, 0);
    const QRect contents = lineEditContents(edit_);
    if (suffix_) {
        // The suffix hugs the value ("25%"), stopping short of the arrow.
        const int textEnd =
            contents.left() + kTextInset + edit_->fontMetrics().horizontalAdvance(edit_->text());
        const int limit = contents.right() + 1 - arrowWidth - suffix_->width();
        suffix_->move(std::min(textEnd, limit), (edit_->height() - suffix_->height()) / 2);
        suffix_->raise();
    }
    if (arrow_) {
        const QRect frame = lineEditFrame(edit_);
        arrow_->setGeometry(frame.right() + 1 - arrowWidth, frame.top(), arrowWidth,
                            frame.height());
        arrow_->raise();
    }
}

void NumericField::showPopup()
{
    if (!popup_ || !slider_) {
        return;
    }
    popup_->adjustSize();
    const int x = (width() - popup_->width()) / 2;
    popup_->move(mapToGlobal(QPoint(x, height())));
    popup_->show();
    if (arrow_) {
        arrow_->setChecked(true);
    }
    // The popup window may never take keyboard focus, but focusing the slider
    // keeps the native step behaviour when it does.
    slider_->setFocus(Qt::PopupFocusReason);
}

bool NumericField::eventFilter(QObject* watched, QEvent* event)
{
    if (watched == edit_) {
        switch (event->type()) {
        case QEvent::Resize:
            layoutInset();
            return QWidget::eventFilter(watched, event);
        case QEvent::FontChange:
        case QEvent::StyleChange:
            reserveWidth();
            return QWidget::eventFilter(watched, event);
        case QEvent::ShortcutOverride:
        case QEvent::KeyPress: {
            // Up/Down step the value (Shift 10x, Ctrl 0.1x) and commit, ahead
            // of any arrow-key shortcut.
            auto* key = static_cast<QKeyEvent*>(event);
            if (key->key() != Qt::Key_Up && key->key() != Qt::Key_Down) {
                break;
            }
            event->accept();
            if (event->type() == QEvent::KeyPress) {
                bool ok = false;
                const double typed = edit_->text().toDouble(&ok);
                const double delta = modifierScale(key->modifiers()) * config_.step;
                applyUserValue((ok ? typed : value_)
                               + (key->key() == Qt::Key_Up ? delta : -delta));
                commitPending();
            }
            return true;
        }
        default:
            break;
        }
    }
    if (watched == popup_) {
        if (event->type() == QEvent::Hide) {
            if (arrow_) {
                arrow_->setChecked(false);
            }
            commitPending();
        } else if (event->type() == QEvent::KeyPress) {
            auto* key = static_cast<QKeyEvent*>(event);
            double next = value_;
            switch (key->key()) {
            case Qt::Key_Left:
                next -= config_.step;
                break;
            case Qt::Key_Right:
                next += config_.step;
                break;
            case Qt::Key_Home:
                next = config_.minimum;
                break;
            case Qt::Key_End:
                next = config_.maximum;
                break;
            case Qt::Key_PageUp:
                next += config_.page;
                break;
            case Qt::Key_PageDown:
                next -= config_.page;
                break;
            default:
                return QWidget::eventFilter(watched, event);
            }
            applyUserValue(next);
            commitPending();
            return true;
        }
    }

    const bool isHandle = watched == label_ || watched == edit_ || watched == suffix_;
    if (!isHandle) {
        return QWidget::eventFilter(watched, event);
    }
    if (event->type() == QEvent::MouseButtonPress) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if (mouse->button() == Qt::LeftButton && isEnabled()) {
            scrubbing_ = false;
            scrubOrigin_ = mouse->globalPosition().toPoint();
            scrubStart_ = value_;
        }
    } else if (event->type() == QEvent::MouseMove) {
        auto* mouse = static_cast<QMouseEvent*>(event);
        if ((mouse->buttons() & Qt::LeftButton) && isEnabled()) {
            const int delta = mouse->globalPosition().toPoint().x() - scrubOrigin_.x();
            if (scrubbing_ || qAbs(delta) >= 3) {
                scrubbing_ = true;
                const double scale =
                    modifierScale(mouse->modifiers()) * config_.step;
                applyUserValue(scrubStart_ + delta * scale);
                return true;
            }
        }
    } else if (event->type() == QEvent::MouseButtonRelease && scrubbing_) {
        scrubbing_ = false;
        commitPending();
        return true;
    }
    return QWidget::eventFilter(watched, event);
}

} // namespace pictura
