// Included by tst_image_adjustments.cpp.
void ImageAdjustmentsTest::shadowsHighlightsAdvanced()
{
    QVERIFY(openImage(QColor(62, 40, 30)));
    const auto makeDialog = [this]() {
        auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("shadows-highlights"), view_,
                                                    block("shadows-highlights"), QRect());
        dialog->show();
        return dialog;
    };
    const auto spinFor = [](const pictura::AdjustmentDialog& dialog, const QString& key) {
        const auto* slider = dialog.controlForTest(key);
        return slider ? slider->parentWidget()->findChild<QDoubleSpinBox*>() : nullptr;
    };
    auto dialog = makeDialog();
    auto* more = dialog->findChild<QCheckBox*>(QStringLiteral("shadowHighlightMoreOptions"));
    QVERIFY(more);
    auto* correction = spinFor(*dialog, QStringLiteral("colorCorrection"));
    QVERIFY(correction);
    auto* black = spinFor(*dialog, QStringLiteral("blackClip"));
    QVERIFY(black);
    QVERIFY(!more->isChecked() && !correction->isVisible());
    QVERIFY(dialog->controlForTest(QStringLiteral("shadowAmount"))->isVisible());
    QVERIFY(dialog->controlForTest(QStringLiteral("highlightAmount"))->isVisible());
    more->setChecked(true);
    QVERIFY(correction->isVisible());
    QVERIFY(!dialog->controlForTest(QStringLiteral("brightness")));
    for (const char* key : {"shadowWidth", "shadowRadius", "highlightWidth", "highlightRadius",
                            "midtoneContrast", "blackClip", "whiteClip"}) {
        QWidget* control = dialog->controlForTest(QLatin1String(key));
        QVERIFY(control && control->isVisible());
    }
    for (const double clip : {0.01, 0.02, 10.0, 0.01}) {
        black->setValue(clip);
        const QByteArray current = dialog->block();
        const auto rows = pictura::image_adjustment_page(
            ::rust::Slice<const std::uint8_t>(
                reinterpret_cast<const std::uint8_t*>(current.constData()),
                std::size_t(current.size())), false);
        bool found = false;
        for (const QString& row : rows) {
            const QStringList cells = row.split(QLatin1Char('\t'));
            if (cells.size() > 4 && cells[1] == QLatin1String("blackClip")) {
                QCOMPARE(cells[4].toDouble(), clip);
                found = true;
                break;
            }
        }
        QVERIFY(found);
    }
    correction->setValue(-80.0);
    waitForPreview(*view_);
    const QRgb lowChroma = view_->composite_argb(5, 5);
    correction->setValue(80.0);
    waitForPreview(*view_);
    const QRgb highChroma = view_->composite_argb(5, 5);
    QVERIFY(qRed(highChroma) - qBlue(highChroma) > qRed(lowChroma) - qBlue(lowChroma));
    dialog->accept();
    QCOMPARE(dialog->result(), int(QDialog::Accepted));
    QCOMPARE(view_->composite_argb(5, 5), highChroma);
    dialog.reset();

    QVERIFY(pictura::convert_image_mode(*view_, QStringLiteral("grayscale")));
    dialog = makeDialog();
    more = dialog->findChild<QCheckBox*>(QStringLiteral("shadowHighlightMoreOptions"));
    QVERIFY(more);
    more->setChecked(true);
    auto* brightness = spinFor(*dialog, QStringLiteral("brightness"));
    QVERIFY(brightness && brightness->isVisible());
    QVERIFY(!dialog->controlForTest(QStringLiteral("colorCorrection")));
    brightness->setValue(-30.0);
    waitForPreview(*view_);
    const QRgb dim = view_->composite_argb(5, 5);
    brightness->setValue(30.0);
    waitForPreview(*view_);
    const QRgb bright = view_->composite_argb(5, 5);
    QVERIFY(qGray(bright) > qGray(dim));
    dialog->accept();
    QCOMPARE(dialog->result(), int(QDialog::Accepted));
    QCOMPARE(view_->composite_argb(5, 5), bright);
}

void ImageAdjustmentsTest::shadowsHighlightsSelection()
{
    QVERIFY(openRamp(25, 90));
    QVERIFY(view_->select_rect(0, 0, 10, 20, QStringLiteral("new"), 0.0));
    const QRgb inside = view_->composite_argb(5, 5);
    const QRgb outside = view_->composite_argb(15, 5);
    const int history = view_->history_count();
    auto dialog = pictura::makeAdjustmentDialog(QStringLiteral("shadows-highlights"), view_,
                                                block("shadows-highlights"), QRect());
    waitForPreview(*view_);
    const QRgb preview = view_->composite_argb(5, 5);
    QVERIFY(qGray(preview) > qGray(inside));
    QCOMPARE(view_->composite_argb(15, 5), outside);
    dialog->accept();
    QCOMPARE(dialog->result(), int(QDialog::Accepted));
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->composite_argb(5, 5), preview);
    QCOMPARE(view_->composite_argb(15, 5), outside);
    QVERIFY(view_->undo());
    QCOMPARE(view_->composite_argb(5, 5), inside);
    QCOMPARE(view_->composite_argb(15, 5), outside);
}

void ImageAdjustmentsTest::shadowsHighlightsDepth()
{
    QVERIFY(openImage(QColor(40, 45, 50)));
    QAction* action = leaf(QStringLiteral("Shadows/Highlights"));
    QVERIFY(action && action->isEnabled());
    QVERIFY(pictura::convert_image_depth(*view_, 16));
    window_->registry()->refresh();
    QVERIFY(action->isEnabled());
    QVERIFY(pictura::convert_image_depth(*view_, 32));
    window_->registry()->refresh();
    QVERIFY(!action->isEnabled());
    const auto expanded = block("shadows-highlights");
    const ::rust::Slice<const std::uint8_t> slice(
        reinterpret_cast<const std::uint8_t*>(expanded.constData()), std::size_t(expanded.size()));
    const int history = view_->history_count();
    QVERIFY(!pictura::image_adjust_apply(*view_, slice, QStringLiteral("Shadows/Highlights")));
    QCOMPARE(view_->history_count(), history);
}
