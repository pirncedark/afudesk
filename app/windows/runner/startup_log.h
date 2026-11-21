#ifndef RUNNER_STARTUP_LOG_H_
#define RUNNER_STARTUP_LOG_H_

#include <string>

namespace afudesk {

void StartupLogReset();
void StartupLog(const std::string& msg);
void StartupErrorLog(const std::string& msg);
bool RedirectStdioIfRequested();

}  // namespace afudesk

#endif  // RUNNER_STARTUP_LOG_H_
