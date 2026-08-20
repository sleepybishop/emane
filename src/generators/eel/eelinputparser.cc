#include "eelinputparser.h"
#include "emane/generators/eel/formatexception.h"
#include <cstdlib>
#include <cstring>

extern "C" {
    bool emane_rs_eel_input_parser_parse(
        const char* input,
        float* f_event_time,
        char** s_event_type_out,
        char** s_module_id_out,
        char*** args_out,
        size_t* args_len_out,
        char** error_out
    );
    void emane_rs_eel_input_parser_free_strings(
        char* s_event_type,
        char* s_module_id,
        char** args,
        size_t args_len,
        char* error
    );
}

EMANE::Generators::EEL::InputParser::InputParser(){}

EMANE::Generators::EEL::InputParser::~InputParser(){}

bool EMANE::Generators::EEL::InputParser::parse(const std::string & sInput,
                                                float & fEventTime,
                                                std::string &sEventType,
                                                std::string &sModuleId,
                                                InputArguments & inputArguments)
{ 
    char* s_event_type_out = nullptr;
    char* s_module_id_out = nullptr;
    char** args_out = nullptr;
    size_t args_len_out = 0;
    char* error_out = nullptr;

    bool res = emane_rs_eel_input_parser_parse(
        sInput.c_str(),
        &fEventTime,
        &s_event_type_out,
        &s_module_id_out,
        &args_out,
        &args_len_out,
        &error_out
    );

    if (error_out != nullptr) {
        std::string err(error_out);
        emane_rs_eel_input_parser_free_strings(nullptr, nullptr, nullptr, 0, error_out);
        throw FormatException(err);
    }

    if (res) {
        sEventType = s_event_type_out;
        sModuleId = s_module_id_out;
        inputArguments.clear();
        for (size_t i = 0; i < args_len_out; ++i) {
            inputArguments.push_back(args_out[i]);
        }
    }

    emane_rs_eel_input_parser_free_strings(s_event_type_out, s_module_id_out, args_out, args_len_out, nullptr);
    return res;
}

std::string EMANE::Generators::EEL::InputParser::getNextArgument(const std::string & sInput,
                                                                 size_t & posStart)
{
    return ""; // Obsoleted by Rust parser
}
