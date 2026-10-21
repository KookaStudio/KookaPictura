#include "placeholder_panel.h"

#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>
#include <QtWidgets/QWidget>

namespace pictura {

PlaceholderPanel::PlaceholderPanel(const QString& title, const QString& message, QWidget* parent)
    : QWidget(parent)
{
    setWindowTitle(title);

    QWidget* body = this;
    auto* layout = new QVBoxLayout(body);
    auto* label = new QLabel(message.isEmpty() ? QStringLiteral("No ") + title : message, body);
    label->setAlignment(Qt::AlignCenter);
    label->setEnabled(false);
    layout->addWidget(label, 1, Qt::AlignCenter);
}

} // namespace pictura

#include "moc_placeholder_panel.cpp"
