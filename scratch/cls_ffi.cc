
extern "C" {
    void emane_c_statistic_numeric_add(void* ptr, uint64_t val) {
        if(ptr) {
            auto stat = static_cast<EMANE::StatisticNumeric<std::uint64_t>*>(ptr);
            *stat += val;
        }
    }
    void emane_c_statistic_numeric_set_avg(void* ptr, float val) {
        if(ptr) {
            auto stat = static_cast<EMANE::StatisticNumeric<float>*>(ptr);
            *stat = val;
        }
    }
    void emane_c_statistic_table_set_cell(void* ptr, uint16_t row, int col, uint64_t val) {
        if(ptr) {
            auto table = static_cast<EMANE::StatisticTable<EMANE::NEMId>*>(ptr);
            table->setCell(row, col, EMANE::Any{val});
        }
    }
    void emane_c_statistic_table_add_row(void* ptr, uint16_t row, size_t num_cols) {
        if(ptr) {
            auto table = static_cast<EMANE::StatisticTable<EMANE::NEMId>*>(ptr);
            std::vector<EMANE::Any> v{EMANE::Any{row}};
            v.insert(v.end(), num_cols - 1, EMANE::Any{std::uint64_t{0}});
            table->addRow(row, v);
        }
    }
}
