# Apto-TailBench-Applications

A wrapper around Tailbench applications that utilizes the overall runtime system.

For any other application or benchmark suite, this layer needs to be updated accordingly.  

The wrapper sets up adaptation using **apto** (i.e., processing/activation layer) for the application and the system. 
The wrapper then launches the application and the system module. <br>

After every iteration, the module 
reads the start and end timestamps of a served request from the tailbench application using a linux message queue. <br>

Compute latency is calculated using these timestamps and passed to the processing/activation layer, 
that then proceeds with adaptation.
