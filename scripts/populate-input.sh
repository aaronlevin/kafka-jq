#! /bin/bash

counter=0
while [ $counter -le 1000 ]; do
  #./kafka-publish-msg.sh "{\"events\": [{\"id\": \"$(( ( RANDOM % 5 ) +1 ))\", \"count\": $counter\"}]}"
  message="{\"id\": $(( RANDOM % 10 )), \"count\": $counter}"
  echo "publishing: $message"
  ./kafka-publish-msg.sh $message
  counter=$[$counter+1]
done

