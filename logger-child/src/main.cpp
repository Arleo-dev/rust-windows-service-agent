#include <chrono>
#include <ctime>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <string>

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: logger_child <METRIC_STRING> [LOG_FILE_PATH]" << std::endl;
        return 1;
    }

    std::string metric_data = argv[1];
    std::string log_file_path = (argc >= 3) ? argv[2] : "metrics.log";

    std::cout << "[CHILD LOG] " << metric_data << std::endl;

    std::ofstream log_file(log_file_path, std::ios_base::app);
    if (!log_file.is_open()) {
        std::cerr << "Error: Could not open log file (Access Denied or Path Invalid)." << std::endl;
        return 2;
    }

    auto now = std::chrono::system_clock::now();
    std::time_t time_now = std::chrono::system_clock::to_time_t(now);
    std::tm tm_buf{};

#if defined(_WIN32) || defined(_WIN64)
    localtime_s(&tm_buf, &time_now);
#else
    localtime_r(&time_now, &tm_buf);
#endif

    log_file << "[" << std::put_time(&tm_buf, "%Y-%m-%d %H:%M:%S") << "] " 
             << metric_data << std::endl;
    log_file.close();

    return 0;
}