#pragma once

#include "selftest_run.h"

#include <QtCore/QString>
#include <QtGui/QImage>
#include <QtWidgets/QApplication>

namespace pictura {
class PicturaMainWindow;
class PictureView;
}

int runSelfTest(QApplication& app, bool headless, const QString& psdPath,
                pictura::PicturaMainWindow& frame, pictura::PictureView* view,
                QImage image, bool documentMode, int gpu);
