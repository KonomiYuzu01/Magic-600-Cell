#pragma once
#include "protocol.h"

#include <functional>

namespace sb {
// Exit code of a process that ended after a drain it could not confirm.
constexpr int abandoned_exit = 3;
Result run_mode(const Options& options, const std::string& mode);
int run_child(const Options& options);
// Receives the failing mode's result just before such a process ends.
void set_abandon_report(std::function<void(const Result&)> report);
}
