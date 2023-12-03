import iperf3
client = iperf3.Client()
client.blksize = "2.0G"
client.bind_address = '10.0.0.1'
client.server_hostname = '10.0.0.2'
client.port = 6969
client.zerocopy = True
client.verbose = True
client.reverse = False
client.json_output = True
result = client.run()
print(f"Bandwidth = {result.Mbps}")
