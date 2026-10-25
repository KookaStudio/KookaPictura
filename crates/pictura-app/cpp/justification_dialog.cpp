#include "justification_dialog.h"

#include "api/type_api.h"

#include <QtWidgets/QDialogButtonBox>
#include <QtWidgets/QDoubleSpinBox>
#include <QtWidgets/QGridLayout>
#include <QtWidgets/QLabel>
#include <QtWidgets/QVBoxLayout>

namespace pictura {

namespace {

QDoubleSpinBox* spin(QWidget* parent, const char* name, double minimum, double maximum,
                     const QString& suffix)
{
    auto* box = new QDoubleSpinBox(parent);
    box->setObjectName(QLatin1String(name));
    box->setRange(minimum, maximum);
    box->setDecimals(0);
    box->setSuffix(suffix);
    return box;
}

} // namespace

JustificationDialog::JustificationDialog(const ParagraphSetting& setting, QWidget* parent)
    : QDialog(parent)
    , setting_(setting)
{
    setWindowTitle(tr("Justification"));

    auto* root = new QVBoxLayout(this);
    auto* grid = new QGridLayout();
    grid->addWidget(new QLabel(QString(), this), 0, 0);
    grid->addWidget(new QLabel(tr("Minimum"), this), 0, 1);
    grid->addWidget(new QLabel(tr("Desired"), this), 0, 2);
    grid->addWidget(new QLabel(tr("Maximum"), this), 0, 3);

    const struct Row {
        const char* label;
        const char* names[3];
        double minimum;
        double maximum;
        double values[3];
    } rows[] = {
        {QT_TR_NOOP("Word Spacing:"),
         {"justificationWordMin", "justificationWordDesired", "justificationWordMax"},
         0.0,
         1000.0,
         {setting.word_spacing_min * 100.0, setting.word_spacing_desired * 100.0,
          setting.word_spacing_max * 100.0}},
        {QT_TR_NOOP("Letter Spacing:"),
         {"justificationLetterMin", "justificationLetterDesired", "justificationLetterMax"},
         -100.0,
         500.0,
         {setting.letter_spacing_min * 100.0, setting.letter_spacing_desired * 100.0,
          setting.letter_spacing_max * 100.0}},
        {QT_TR_NOOP("Glyph Scaling:"),
         {"justificationGlyphMin", "justificationGlyphDesired", "justificationGlyphMax"},
         50.0,
         200.0,
         {setting.glyph_spacing_min * 100.0, setting.glyph_spacing_desired * 100.0,
          setting.glyph_spacing_max * 100.0}},
    };

    int row = 1;
    int index = 0;
    for (const Row& spec : rows) {
        grid->addWidget(new QLabel(tr(spec.label), this), row, 0);
        for (int column = 0; column < 3; ++column) {
            auto* box = spin(this, spec.names[column], spec.minimum, spec.maximum,
                             QStringLiteral("%"));
            box->setValue(spec.values[column]);
            fields_[std::size_t(index++)] = box;
            grid->addWidget(box, row, column + 1);
        }
        ++row;
    }
    grid->addWidget(new QLabel(tr("Auto Leading:"), this), row, 0);
    auto* autoLeading = spin(this, "justificationAutoLeading", 0.0, 1000.0, QStringLiteral("%"));
    autoLeading->setValue(setting.auto_leading);
    fields_[9] = autoLeading;
    grid->addWidget(autoLeading, row, 1);

    root->addLayout(grid);
    auto* buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
    connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
    connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    root->addWidget(buttons);
}

ParagraphSetting JustificationDialog::setting() const
{
    ParagraphSetting out = setting_;
    out.word_spacing_min = fields_[0]->value() / 100.0;
    out.word_spacing_desired = fields_[1]->value() / 100.0;
    out.word_spacing_max = fields_[2]->value() / 100.0;
    out.letter_spacing_min = fields_[3]->value() / 100.0;
    out.letter_spacing_desired = fields_[4]->value() / 100.0;
    out.letter_spacing_max = fields_[5]->value() / 100.0;
    out.glyph_spacing_min = fields_[6]->value() / 100.0;
    out.glyph_spacing_desired = fields_[7]->value() / 100.0;
    out.glyph_spacing_max = fields_[8]->value() / 100.0;
    out.auto_leading = fields_[9]->value();
    return out;
}

} // namespace pictura

#include "moc_justification_dialog.cpp"
