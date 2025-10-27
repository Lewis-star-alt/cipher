use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use anyhow::{Result, Context};


#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Файл с алфавитом шифрования в формате "ключ = значение" (пробелы вокруг = разрешены)
    #[arg(short, long)]
    alphabet: String,

    /// Текст для обработки (не указывайте, если используете --input)
    text: Option<String>,

    /// Файл для чтения входного текста
    #[arg(short, long, conflicts_with = "text")]
    input: Option<String>,

    /// Зашифровать текст
    #[arg(short, long, conflicts_with = "decrypt")]
    encrypt: bool,

    /// Расшифровать текст
    #[arg(short, long, conflicts_with = "encrypt")]
    decrypt: bool,

    /// Файл для сохранения результата (если не указан, результат выводится на экран)
    #[arg(short, long)]
    output: Option<String>,

    /// Добавить результат в конец файла (вместо перезаписи)
    #[arg(short = 'A', long, requires = "output")]
    append: bool,
}

#[derive(Debug)]
struct Cipher {
    encrypt_map: HashMap<char, char>,
    decrypt_map: HashMap<char, char>,
}

impl Cipher {
    fn from_file(filename: &str) -> Result<Self> {
        let content = fs::read_to_string(filename)
            .with_context(|| format!("Не удалось прочитать файл: {}", filename))?;
        
        let mut encrypt_map = HashMap::new();
        let mut decrypt_map = HashMap::new();

        for (line_number, line) in content.lines().enumerate() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let equals_pos = match line.find('=') {
                Some(pos) => pos,
                None => anyhow::bail!(
                    "Строка {}: отсутствует знак равенства в '{}'", 
                    line_number + 1, 
                    line
                ),
            };

            let key_part = &line[..equals_pos].trim();
            let value_part = &line[equals_pos + 1..].trim();

            if key_part.is_empty() {
                anyhow::bail!(
                    "Строка {}: пустой ключ в '{}'", 
                    line_number + 1, 
                    line
                );
            }
            if value_part.is_empty() {
                anyhow::bail!(
                    "Строка {}: пустое значение в '{}'", 
                    line_number + 1, 
                    line
                );
            }

            let original = key_part.chars().next()
                .with_context(|| format!("Строка {}: не удалось извлечь ключ", line_number + 1))?;
            let substituted = value_part.chars().next()
                .with_context(|| format!("Строка {}: не удалось извлечь значение", line_number + 1))?;

            if encrypt_map.contains_key(&original) {
                anyhow::bail!(
                    "Строка {}: дублирующийся ключ '{}'", 
                    line_number + 1, 
                    original
                );
            }
            if decrypt_map.contains_key(&substituted) {
                anyhow::bail!(
                    "Строка {}: дублирующееся значение '{}'", 
                    line_number + 1, 
                    substituted
                );
            }

            encrypt_map.insert(original, substituted);
            decrypt_map.insert(substituted, original);
        }

        Ok(Cipher {
            encrypt_map,
            decrypt_map,
        })
    }

    fn encrypt(&self, text: &str) -> String {
        text.chars()
            .map(|c| *self.encrypt_map.get(&c).unwrap_or(&c))
            .collect()
    }

    fn decrypt(&self, text: &str) -> String {
        text.chars()
            .map(|c| *self.decrypt_map.get(&c).unwrap_or(&c))
            .collect()
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Проверяем, что указан либо текст, либо входной файл
    let input_text = match (&args.text, &args.input) {
        (Some(text), None) => text.clone(),
        (None, Some(input_file)) => {
            fs::read_to_string(input_file)
                .with_context(|| format!("Не удалось прочитать входной файл: {}", input_file))?
        }
        (Some(_), Some(_)) => {
            anyhow::bail!("Нельзя одновременно использовать --input и текстовый аргумент");
        }
        (None, None) => {
            anyhow::bail!("Не указан текст для обработки. Используйте текстовый аргумент или --input");
        }
    };

    let cipher = Cipher::from_file(&args.alphabet)?;

    let result = if args.decrypt {
        cipher.decrypt(&input_text)
    } else {
        cipher.encrypt(&input_text)
    };

    match &args.output {
        Some(output_file) => {
            if args.append {
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(output_file)
                    .with_context(|| format!("Не удалось открыть файл для добавления: {}", output_file))?;
                
                writeln!(file, "{}", result)
                    .with_context(|| format!("Не удалось записать в файл: {}", output_file))?;
                
                println!("Результат добавлен в файл: {}", output_file);
            } else {
                // Режим перезаписи файла
                fs::write(output_file, &result)
                    .with_context(|| format!("Не удалось записать результат в файл: {}", output_file))?;
                println!("Результат сохранен в файл: {}", output_file);
            }
        }
        None => {
            println!("{}", result);
        }
    }

    Ok(())
}
