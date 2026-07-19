FROM python:3.11-slim
RUN pip install --no-cache-dir pip setuptools wheel
WORKDIR /opt/codexglue
COPY run_eval_stub.py /opt/codexglue/run_eval_stub.py
# Adapter image is rebuilt after upstream pin changes; dataset mounted at /data.
CMD ["python3", "/opt/codexglue/run_eval_stub.py"]
