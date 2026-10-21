#include "percent_field.h"

namespace pictura {

PercentField::PercentField(const QString& label, QWidget* parent)
    : NumericField(label,
                   NumericFieldConfig{0.0, 100.0, 1.0, 10.0, 0, QStringLiteral("%"), true,
                                      QStringLiteral("percent"),
                                      QStringLiteral("percentField")},
                   parent)
{
}

} // namespace pictura

#include "moc_percent_field.cpp"
