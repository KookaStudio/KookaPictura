void ImageAdjustmentsTest::vibranceAutoUsesOpeningRaster()
{
    QVERIFY(openImage(QColor(105, 125, 135)));
    const QRgb before = view_->composite_argb(5, 5);
    const int history = view_->history_count();
    const auto dialog = pictura::makeAdjustmentDialog(
        QStringLiteral("vibrance"), view_, block("vibrance"), QRect());
    auto* amount = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("vibrance")));
    auto* saturation = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("saturation")));
    auto* button = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"));
    QVERIFY(amount && saturation && button);
    waitForPreview(*view_);
    button->click();
    QVERIFY(amount->value() > 0 && amount->value() <= 35);
    QCOMPARE(saturation->value(), 0);
    waitForPreview(*view_);
    const QRgb preview = view_->composite_argb(5, 5);
    QVERIFY(qBlue(preview) - qRed(preview) > qBlue(before) - qRed(before));
    const QByteArray recommendation = dialog->block();
    button->click();
    QCOMPARE(dialog->block(), recommendation);
    QCOMPARE(view_->history_count(), history);
    dialog->reject();
    QCOMPARE(view_->composite_argb(5, 5), before);
    QCOMPARE(view_->history_count(), history);
}

void ImageAdjustmentsTest::vibranceAutoIgnoresOpeningPreview()
{
    QVERIFY(openImage(QColor(105, 125, 135)));
    const QRgb before = view_->composite_argb(5, 5);
    QByteArray expected;
    {
        const auto dialog = pictura::makeAdjustmentDialog(
            QStringLiteral("vibrance"), view_, block("vibrance"), QRect());
        auto* button = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"));
        QVERIFY(button);
        button->click();
        expected = dialog->block();
        QVERIFY(expected != block("vibrance"));
        dialog->reject();
    }

    const QByteArray neutral = block("vibrance");
    const auto vivid = pictura::image_adjustment_set(
        {reinterpret_cast<const std::uint8_t*>(neutral.constData()), std::size_t(neutral.size())},
        QStringLiteral("vibrance"), 100, false);
    QVERIFY(!vivid.empty());
    const QByteArray opening(reinterpret_cast<const char*>(vivid.data()), qsizetype(vivid.size()));
    const auto dialog = pictura::makeAdjustmentDialog(
        QStringLiteral("vibrance"), view_, opening, QRect());
    auto* button = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"));
    QVERIFY(button);
    waitForPreview(*view_);
    QVERIFY(view_->composite_argb(5, 5) != before);
    button->click();
    QCOMPARE(dialog->block(), expected);
    dialog->reject();
}

void ImageAdjustmentsTest::vibranceAutoReplacesHigherValue()
{
    QVERIFY(openImage(QColor(105, 125, 135)));
    const auto dialog = pictura::makeAdjustmentDialog(
        QStringLiteral("vibrance"), view_, block("vibrance"), QRect());
    auto* amount = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("vibrance")));
    auto* saturation = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("saturation")));
    auto* button = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"));
    QVERIFY(amount && saturation && button);
    amount->setValue(100);
    saturation->setValue(-100);
    button->click();
    QVERIFY(amount->value() > 0 && amount->value() <= 35);
    QCOMPARE(saturation->value(), -100);
    waitForPreview(*view_);
    const QRgb preview = view_->composite_argb(5, 5);
    QCOMPARE(qRed(preview), qGreen(preview));
    QCOMPARE(qGreen(preview), qBlue(preview));
    dialog->reject();
}

void ImageAdjustmentsTest::vibranceAutoUsesSelection()
{
    const QColor muted(105, 125, 135);
    const QColor vivid(20, 235, 20);
    QImage image(20, 20, QImage::Format_RGB32);
    image.fill(vivid);
    for (int y = 0; y < image.height(); ++y) {
        image.setPixelColor(0, y, muted);
        image.setPixelColor(1, y, muted);
    }
    const QString path = dir_.filePath(QStringLiteral("vibrance-selection.png"));
    QVERIFY(image.save(path) && window_->openDocumentAtPath(path));
    QCoreApplication::processEvents();
    view_ = window_->activeView();
    QVERIFY(view_ && view_->select_rect(0, 0, 2, 20, QStringLiteral("new"), 0.0));
    const int history = view_->history_count();

    const auto dialog = pictura::makeAdjustmentDialog(
        QStringLiteral("vibrance"), view_, block("vibrance"), QRect());
    auto* amount = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("vibrance")));
    auto* button = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"));
    QVERIFY(amount && button);
    button->click();
    QVERIFY(amount->value() > 0);
    dialog->accept();
    QCOMPARE(dialog->result(), int(QDialog::Accepted));
    QCOMPARE(view_->history_count(), history + 1);
    QCOMPARE(view_->history_label(view_->history_index()), QStringLiteral("Vibrance"));
    const QRgb adjusted = view_->composite_argb(0, 5);
    QVERIFY(qBlue(adjusted) - qRed(adjusted) > muted.blue() - muted.red());
    QCOMPARE(view_->composite_argb(5, 5), vivid.rgb());
}

void ImageAdjustmentsTest::vibranceAutoSkipsGrayAndVivid()
{
    for (const QColor color : {QColor(125, 125, 125), QColor(20, 235, 20)}) {
        QVERIFY(openImage(color));
        const int history = view_->history_count();
        auto dialog = pictura::makeAdjustmentDialog(
            QStringLiteral("vibrance"), view_, block("vibrance"), QRect());
        auto* amount = qobject_cast<QSpinBox*>(dialog->controlForTest(QStringLiteral("vibrance")));
        auto* button = dialog->findChild<QPushButton*>(QStringLiteral("adjustmentButtonAuto"));
        QVERIFY(amount && button);
        amount->setValue(-20);
        const QByteArray edited = dialog->block();
        button->click();
        QCOMPARE(dialog->block(), edited);
        QCOMPARE(amount->value(), -20);
        QCOMPARE(view_->history_count(), history);
        dialog->reject();
        dialog.reset();
        cleanup();
    }
}


