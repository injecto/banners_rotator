# Баннерокрутилка

Читает из CSV баннеры и раздает их по HTTP.

Формат CSV:
```
http://banners.com/banner2.jpg;3300;show;britain;benny hill;sketches;tv
```
Первая строка содержит URL картинки-баннера. Вторая -- доступное количество показов.
Третья и следующие строки -- теги баннера, каждый баннер должен иметь минимимум один тег.

Пример запросов баннера:
```shell script
# Запрос по любому из тегов
> http GET :8080 category==flight category==sandbox
HTTP/1.1 200 OK
content-length: 69
date: Sun, 01 Dec 2019 19:01:34 GMT

<html><body><img src="http://banners.com/banner3.jpg"/></body></html>

# Запрос произвольного баннера
> dev http GET :8080
...

# Запрос по несуществующему тегу ИЛИ в случае исчерпания количества показов
> dev http GET :8080 category==flight                  
  HTTP/1.1 204 No Content
  content-length: 0
  date: Mon, 02 Dec 2019 07:54:37 GMT
```

Выдача баннеров сделана таким образом, чтобы уменьшить вероятность возвращения пустого ответа.
То есть при выборке учитывается количество показов.

## Сборка
```shell script
> cargo build --release
...
> ./target/release/banners_rotator -h
  Banners rotator 
  
  USAGE:
      banners_rotator [OPTIONS] <FILE>
  
  FLAGS:
      -h, --help       Prints help information
      -V, --version    Prints version information
  
  OPTIONS:
      -p, --port <http_port>    Listening HTTP port [default: 8080]
  
  ARGS:
      <FILE>    Banners config as CSV
```

## Запуск
```shell script
> .banners_rotator examples/banners10k.csv 
InMemoryStore[banners: 10000, categories: 51375] loaded
Start listening on 0.0.0.0:8080
```

Примеры CSV лежат в `examples/`. Там же лежит скрипт, с помощью которого можно сгенерировать
CSV произвольного размера:
```shell script
 > ./generate_example.py 10000 > banners10k.csv
```
