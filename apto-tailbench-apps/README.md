# Apto TailBench Applications

A tight wrapper around Tailbench applications that utilizes the adaptation pipeline.

The wrapper sets up adaptation using apto for the application and the underlying system.
The wrapper then launches the application and the system module. After every iteration the it reads the start and end timestamp of serving a request from the tailbench application using a linux message queue. 
Compute latency is calculated using these timestamps and passed to apto which then takes over adaptation.
