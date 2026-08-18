import re

with open('src/libemane/commonlayerstatistics.cc', 'r') as f:
    cc_content = f.read()

# Extract variables
numeric_vars = re.findall(r'StatisticNumeric<Counter> \* (pNum[a-zA-Z_0-9]+);', cc_content)
table_vars = re.findall(r'StatisticTable<NEMId> \* (pStatistic[a-zA-Z_0-9]+Table_);', cc_content)

print(f"numeric_vars = {numeric_vars}")
print(f"table_vars = {table_vars}")
