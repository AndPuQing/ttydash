# ttydash

![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/550W-HOST/ttydash/ci.yml?style=flat-square&logo=github)
 ![Crates.io Version](https://img.shields.io/crates/v/ttydash?style=flat-square&logo=rust)
 ![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/ttydash?style=flat-square)
[![dependency status](https://deps.rs/repo/github/550w-host/ttydash/status.svg?style=flat-square)](https://deps.rs/repo/github/550w-host/ttydash)
![Crates.io License](https://img.shields.io/crates/l/ttydash?style=flat-square) ![Crates.io Size](https://img.shields.io/crates/size/ttydash?style=flat-square)

## Snapshot

![ttydash](./assets/Snipaste.png)

Auto Layout

![ttydash](./assets/grid.png)

## Installation

```bash
cargo install ttydash
```

## **`ttydash` Usage Guide**

### **Single Line Data Input**

#### **Pure Data Input** (No Title, No Unit)
To input pure data continuously, use the following command:
```bash
while true; echo 1; sleep 0.5; end | ttydash
```

#### **Adding a Title** (Optional)
If you want to add a title to the data chart, just use the `-t` flag:
```bash
while true; echo 1; sleep 0.5; end | ttydash -t "🌟 Title"
```

#### **Adding Units** (Optional)
If each line of data comes with a unit (e.g., "ms"), you can specify the unit with the `-u` flag. 

Example 1️:
```bash
while true; echo 1ms; sleep 0.5; end | ttydash -u ms
```
Example 2️:
```bash
while true; echo 1 ms; sleep 0.5; end | ttydash -u ms
```
👉 Note: The space between the number and the unit is optional.

### ➕ **Multiple Data Points** on the Same Line
To input multiple data points at once, just separate them with a space. For example:
```bash
while true; echo "1 2 3"; sleep 0.5; end | ttydash
```

📊 `ttydash` will plot the data points in the order they are provided!

### 🎯 **Plot Specific Data Points** Using the `-i` Flag
If you only want to plot specific data points, you can use the `-i` flag to select their index. For example:
```bash
while true; echo "1 2 3"; sleep 0.5; end | ttydash -i 1 -i 2
```
In this example, only the data at **index 1** and **index 2** will be plotted.

👉 **Note**: You can switch the sequence of the index as needed. For example:
```bash
ttydash -i 2 -i 1
```
This will plot **index 2** first, followed by **index 1**.

### 💡 **Advanced Data Extraction with Regular Expressions**
For more complex data formats, you can use the `--regex` flag to specify a custom regular expression. The regex must contain a named capture group called `value`.

**Example 1: Extracting a number with a prefix**
If your input is `CPU usage: 80%`, you can extract the value `80` with the following command:
```bash
while true; echo "CPU usage: 80%"; sleep 0.5; end | ttydash --regex "CPU usage: (?<value>\d+)%"
```

**Example 2: Using predefined regular expressions**
You can add, remove, and list predefined regular expressions using the `add`, `remove`, and `list` subcommands.

To add a new regex for extracting the time from the `ping` command:
```bash
ttydash add ping "time=(?<value>\\d+\\.\\d+)\\s+ms"
```

To list all saved regular expressions:
```bash
ttydash list
```

To remove the "ping" regex:
```bash
ttydash remove ping
```

Once a regex is saved, you can use it by name with the `--regex` flag:
```bash
ping google.com | ttydash --regex ping
```

**Example 3: Combining different extraction methods**
You can use multiple extraction flags at the same time. `ttydash` will create a separate chart for each extractor.

```bash
while true; echo "temp=36.6C hum=45%"; sleep 0.5; end | ttydash --regex "temp=(?<value>\d+\.\d+)C" --regex "hum=(?<value>\d+)%"
```

This will create two charts, one for temperature and one for humidity.

### 📈 **Group Chart**

```bash
while true; echo "1 2 3"; sleep 0.5; end | ttydash -g
```

![](./assets/group_chart.png)

## flags

```bash
A terminal-based dashboard for real-time data visualization.

Usage: ttydash [OPTIONS] [COMMAND]

Commands:
  add     Add a new regex to the list of regexes
  remove  Remove a regex from the list of regexes
  list    List all regexes
  help    Print this message or the help of the given subcommand(s)

Options:
      --tick-rate <FLOAT>       Tick rate, i.e. number of ticks per second [default: 4]
  -f, --frame-rate <FLOAT>      Frame rate, i.e. number of frames per second [default: 60]
  -t, --titles <STRING>         Chart title, will be shown at the top of the chart
  -u, --units <UNITS>           Unit to be used in the chart (e.g. "ms", "MB")
  -i, --indices <INT>           Index vector to be used in the chart
      --regex <PATTERN>         Regex to extract values, requires a "value" named capture group
  -g, --group[=<GROUP>]         Group together to show multiple charts in the same window [default: false] [possible values: true, false]
      --update-frequency <INT>  Update frequency, i.e. number of milliseconds between updates [default: 1000]
  -l, --layout <STRING>         Layout of the chart [default: auto] [possible values: horizontal, vertical, auto]
  -h, --help                    Print help
  -V, --version                 Print version
```

## Future Features (TODO)

- [x] **Data Threshold Highlighting**: Allow users to set thresholds. When data exceeds or falls below a certain value, the chart's color will change, useful for monitoring and alerts.
- [ ] **Advanced Layout System**: Building on the existing auto-layout, allow users to define a more complex grid layout through a configuration file, enabling precise control over each chart's position and size.
- [ ] **Data Persistence**: Add a feature to save incoming data to a local file and allow loading of historical data on startup for analysis.
- [ ] **Chart Interaction**: Implement chart zoom and pan functionalities to allow users to analyze historical data in detail.
- [ ] **Runtime Configuration**: Allow users to modify configurations like titles, units, etc., through interactive commands while `ttydash` is running.
- [ ] **Multiple Data Sources**: In addition to `stdin`, support more data sources:
  - [ ] **File Source**: Monitor a file for new content, similar to `tail -f`.
  - [ ] **Command Source**: Periodically execute a command and use its output as a data source.
  - [ ] **Network Source**: Listen on a TCP or UDP port to receive data streams.