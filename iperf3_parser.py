def parse_iperf_data():
    parsed_data = []
    with open('out.txt', 'r') as file:
        for line in file:
            if 'sec' in line:
                parts = line.split()
                interval = parts[2]
                interval = interval.split("-")[1]
                transfer = parts[4] + ' ' + parts[5]
                bitrate = ((parts[6] + ' ' + parts[7]).split(" "))
                if(bitrate[1]=="Kbits/sec"):
                    bitrate = float(bitrate[0])/1000
                elif(bitrate[1]=="Gbits/sec"):
                    bitrate = float(bitrate[0])*1000
                parsed_data.append({'Time': interval, 'Transfer': transfer, 'Bitrate': bitrate})
    return parsed_data

print(parse_iperf_data())

# print(parsed_data[:-1])

def parse_quic_data():
    parsed_data = []
    with open("quic_bandwidth.txt") as file:
        for line in file:
            params = line.split(",")
            data_transferred = params[0]
            time = params[1]
            bandwidth = params[2]
            parsed_data.append({'Time': time, 'Transfer': data_transferred, 'Bitrate': bandwidth})




# parse_iperf_data

            

