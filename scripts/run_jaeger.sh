docker run --rm --name jaeger \
  -p 16686:16686 \
  -p 8317:4317 \
  -p 8318:4318 \
  -p 5778:5778 \
  -p 9411:9411 \
  cr.jaegertracing.io/jaegertracing/jaeger:2.8.0
